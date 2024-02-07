/*
 * Copyright (C) 2023 Vaticle
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::iter;
use std::ops::RangeBounds;

use itertools::Itertools;

use bytes::byte_array::ByteArray;
use durability::SequenceNumber;

use crate::error::MVCCStorageError;
use crate::isolation_manager::CommitRecord;
use crate::key_value::{StorageKey, StorageKeyArray, StorageValue, StorageValueArray};
use crate::keyspace::keyspace::{KEYSPACE_ID_MAX, KeyspaceId};
use crate::MVCCStorage;
use crate::snapshot::buffer::{BUFFER_INLINE_KEY, BUFFER_INLINE_VALUE, KeyspaceBuffer};

pub enum Snapshot<'storage> {
    Read(ReadSnapshot<'storage>),
    Write(WriteSnapshot<'storage>),
}

impl<'storage> Snapshot<'storage> {
    pub fn get<'snapshot>(&'snapshot self, key: &StorageKey<'_, BUFFER_INLINE_KEY>) -> Option<StorageValueArray<BUFFER_INLINE_VALUE>> {
        match self {
            Snapshot::Read(snapshot) => snapshot.get(key),
            Snapshot::Write(snapshot) => snapshot.get(key),
        }
    }

    pub fn iterate_prefix<'this>(&'this self, prefix: &StorageKey<'_, BUFFER_INLINE_KEY>) -> Box<dyn Iterator<Item=Result<(StorageKey<'this, BUFFER_INLINE_KEY>, StorageValue<'this, BUFFER_INLINE_VALUE>), MVCCStorageError>> + 'this> {
        match self {
            Snapshot::Read(snapshot) => Box::new(snapshot.iterate_prefix(prefix)),
            Snapshot::Write(snapshot) => Box::new(snapshot.iterate_prefix(prefix)),
        }
    }
}

pub struct ReadSnapshot<'storage> {
    storage: &'storage MVCCStorage,
    open_sequence_number: SequenceNumber,
}

impl<'storage> ReadSnapshot<'storage> {
    pub(crate) fn new(storage: &'storage MVCCStorage, open_sequence_number: SequenceNumber) -> ReadSnapshot {
        // Note: for serialisability, we would need to register the open transaction to the IsolationManager
        ReadSnapshot {
            storage: storage,
            open_sequence_number: open_sequence_number,
        }
    }

    fn get<'snapshot>(&self, key: &StorageKey<'_, BUFFER_INLINE_KEY>) -> Option<StorageValueArray<BUFFER_INLINE_VALUE>> {
        // TODO: this clone may not be necessary - we could pass a reference up?
        self.storage.get(key, &self.open_sequence_number, |reference| StorageValueArray::new(ByteArray::from(reference)))
    }

    fn iterate_prefix<'this>(&'this self, prefix: &'this StorageKey<'this, BUFFER_INLINE_KEY>) -> impl Iterator<Item=Result<(StorageKey<'this, BUFFER_INLINE_KEY>, StorageValue<'this, BUFFER_INLINE_VALUE>), MVCCStorageError>> + 'this {
        self.storage.iterate_prefix(prefix, &self.open_sequence_number)
            .map(|result| result.map(|(k, v)| {
                (StorageKey::Reference(k), StorageValue::Reference(v))
            }))
    }
}

pub struct WriteSnapshot<'storage> {
    storage: &'storage MVCCStorage,
    // TODO: replace with BTree Left-Right structure to allow concurrent read/write
    buffers: [KeyspaceBuffer; KEYSPACE_ID_MAX],
    open_sequence_number: SequenceNumber,
}

impl<'storage> WriteSnapshot<'storage> {
    pub(crate) fn new(storage: &'storage MVCCStorage, open_sequence_number: SequenceNumber) -> WriteSnapshot {
        storage.isolation_manager.opened(&open_sequence_number);
        WriteSnapshot {
            storage: storage,
            buffers: core::array::from_fn(|_| KeyspaceBuffer::new()),
            open_sequence_number: open_sequence_number,
        }
    }

    /// Insert a key with a new version
    pub fn insert(&self, key: StorageKeyArray<BUFFER_INLINE_KEY>) {
        self.insert_val(key, StorageValueArray::empty())
    }

    pub fn insert_val(&self, key: StorageKeyArray<BUFFER_INLINE_KEY>, value: StorageValueArray<BUFFER_INLINE_VALUE>) {
        let keyspace_id = key.keyspace_id();
        let byte_array = key.into_byte_array();
        self.get_buffer(keyspace_id).insert(byte_array, value);
    }

    /// Insert a key with a new version if it does not already exist.
    /// If the key exists, mark it as a preexisting insertion to escalate to Insert if there is a concurrent Delete.
    pub fn put(&self, key: StorageKeyArray<BUFFER_INLINE_KEY>) {
        self.put_val(key, StorageValueArray::empty())
    }

    pub fn put_val(&self, key: StorageKeyArray<BUFFER_INLINE_KEY>, value: StorageValueArray<BUFFER_INLINE_VALUE>) {
        let buffer = self.get_buffer(key.keyspace_id());
        let existing_buffered = buffer.contains(key.byte_array());
        if !existing_buffered {
            let existing_stored = self.storage.get(
                &StorageKey::Array(key),
                &self.open_sequence_number,
                |reference| {
                    // Only copy if the value is the same
                    if reference == value.bytes() {
                        Some(StorageValueArray::new(ByteArray::from(reference)))
                    } else {
                        None
                    }
                },
            );
            let byte_array = key.into_byte_array();
            if existing_stored.is_some() && existing_stored.as_ref().unwrap().is_some() {
                buffer.insert_preexisting(byte_array, existing_stored.unwrap().unwrap());
            } else {
                buffer.insert(byte_array, value)
            }
        } else {
            // TODO: replace existing buffered write. If it contains a preexisting, we can continue to use it
            todo!()
        }
    }

    /// Insert a delete marker for the key with a new version
    pub fn delete(&self, key: StorageKeyArray<BUFFER_INLINE_KEY>) {
        let keyspace_id = key.keyspace_id();
        let byte_array = key.into_byte_array();
        self.get_buffer(keyspace_id).delete(byte_array);
    }

    /// Get a Value, and mark it as a required key
    pub fn get_required(&self, key: &StorageKey<'_, BUFFER_INLINE_KEY>) -> StorageValueArray<BUFFER_INLINE_VALUE> {
        let buffer = self.get_buffer(key.keyspace_id());
        let existing = buffer.get(key.bytes());
        if existing.is_none() {
            let storage_value = self.storage.get(
                key,
                &self.open_sequence_number,
                |reference| StorageValueArray::new(ByteArray::from(reference)),
            );
            if storage_value.is_some() {
                buffer.require_exists(ByteArray::from(key.bytes()), storage_value.as_ref().unwrap().clone());
                return storage_value.unwrap();
            } else {
                // TODO: what if the user concurrent requires a concept while deleting it in another query
                unreachable!("Require key exists in snapshot or in storage.");
            }
        } else {
            existing.unwrap()
        }
    }

    /// Get the Value for the key, returning an empty Option if it does not exist
    pub fn get(&self, key: &StorageKey<'_, BUFFER_INLINE_KEY>) -> Option<StorageValueArray<BUFFER_INLINE_VALUE>> {
        let existing_value = self.get_buffer(key.keyspace_id()).get(key.bytes());
        existing_value.map_or_else(
            || self.storage.get(key, &self.open_sequence_number, |reference| StorageValueArray::new(ByteArray::from(reference))),
            |existing| Some(existing),
        )
    }

    pub fn iterate_prefix<'this>(&'this self, prefix: &StorageKey<'_, BUFFER_INLINE_KEY>) -> impl Iterator<Item=Result<(StorageKey<'this, BUFFER_INLINE_KEY>, StorageValue<'this, BUFFER_INLINE_VALUE>), MVCCStorageError>> + 'this {
        // let storage_iterator = self.storage.iterate_prefix(prefix, &self.open_sequence_number);
        // let buffered_iterator = self.writes.iterate_prefix(prefix.keyspace_id(), prefix.bytes());
        // storage_iterator.merge_join_by(
        //     buffered_iterator,
        //     |(k1, v1), (k2, v2)| k1.cmp(k2),
        // ).filter_map(|ordering| match ordering {
        //     EitherOrBoth::Both(Ok((k1, v1)), (k2, write2)) => match write2 {
        //         Write::Insert(v2) => Some((k2, v2)),
        //         Write::InsertPreexisting(v2, _) => Some((k2, v2)),
        //         Write::RequireExists(v2) => {
        //             debug_assert_eq!(v1, v2);
        //             Some((k1, v1))
        //         }
        //         Write::Delete => None,
        //     },
        //     EitherOrBoth::Left(Ok((k1, v1))) => Some((k1, v1)),
        //     EitherOrBoth::Right((k2, write2)) => match write2 {
        //         Write::Insert(v2) => Some((k2, v2)),
        //         Write::InsertPreexisting(v2, _) => Some((k2, v2)),
        //         Write::RequireExists(_) => unreachable!("Invalid state: a key required to exist must also exists in Storage."),
        //         Write::Delete => None,
        //     },
        //     EitherOrBoth::Both(Err(_), _) => {
        //         panic!("Unhandled error in iteration")
        //     },
        //     EitherOrBoth::Left(Err(_)) => {
        //         panic!("Unhandled error in iteration")
        //     },
        // })

        // TODO
        iter::empty()
    }

    fn get_buffer(&self, keyspace_id: KeyspaceId) -> &KeyspaceBuffer {
        &self.buffers[keyspace_id as usize]
    }

    pub fn commit(self) {
        self.storage.snapshot_commit(self);
    }

    pub(crate) fn into_commit_record(self) -> CommitRecord {
        CommitRecord::new(
            self.buffers.writes,
            self.open_sequence_number,
        )
    }
}

#[derive(Debug)]
pub struct WriteSnapshotError {
    pub kind: WriteSnapshotErrorKind,
}

#[derive(Debug)]
pub enum WriteSnapshotErrorKind {
    FailedGet { source: MVCCStorageError },
    FailedPut { source: MVCCStorageError },
}

impl Display for WriteSnapshotError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Error for WriteSnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            WriteSnapshotErrorKind::FailedGet { source, .. } => Some(source),
            WriteSnapshotErrorKind::FailedPut { source, .. } => Some(source),
        }
    }
}