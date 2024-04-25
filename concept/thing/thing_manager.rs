/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard};

use bytes::{byte_array::ByteArray, byte_reference::ByteReference, Bytes};
use encoding::{
    graph::{
        thing::{
            edge::{ThingEdgeHas, ThingEdgeHasReverse, ThingEdgeRelationIndex, ThingEdgeRolePlayer},
            vertex_attribute::AttributeVertex,
            vertex_generator::{LongAttributeID, StringAttributeID, ThingVertexGenerator},
            vertex_object::ObjectVertex,
        },
        Typed,
    },
    Keyable,
    layout::prefix::{Prefix, PrefixID},
    value::{decode_value_u64, encode_value_u64, long::Long, string::StringBytes, value_type::ValueType},
};
use encoding::graph::thing::vertex_attribute::AsAttributeID;
use resource::constants::snapshot::BUFFER_KEY_INLINE;
use storage::{
    key_value::StorageKey,
    snapshot::{ReadableSnapshot, WritableSnapshot, write::Write},
};
use storage::key_range::KeyRange;

use crate::{
    ConceptStatus,
    error::{ConceptReadError, ConceptWriteError},
    thing::{
        attribute::{Attribute, AttributeIterator},
        entity::{Entity, EntityIterator},
        object::{HasAttributeIterator, Object},
        ObjectAPI,
        relation::{IndexedPlayersIterator, Relation, RelationIterator, RelationRoleIterator, RolePlayerIterator},
        ThingAPI, value::Value,
    },
    type_::{
        attribute_type::AttributeType, entity_type::EntityType, relation_type::RelationType, role_type::RoleType,
        type_manager::TypeManager, TypeAPI,
    },
};
use crate::thing::attribute::AttributeOwnerIterator;

pub struct ThingManager<Snapshot> {
    snapshot: Arc<Snapshot>,
    vertex_generator: Arc<ThingVertexGenerator>,
    type_manager: Arc<TypeManager<Snapshot>>,
    relation_lock: Mutex<()>,
}

impl<Snapshot: ReadableSnapshot> ThingManager<Snapshot> {
    pub fn new(
        snapshot: Arc<Snapshot>,
        vertex_generator: Arc<ThingVertexGenerator>,
        type_manager: Arc<TypeManager<Snapshot>>,
    ) -> Self {
        ThingManager { snapshot, vertex_generator, type_manager, relation_lock: Mutex::new(()) }
    }

    pub(crate) fn type_manager(&self) -> &TypeManager<Snapshot> {
        &self.type_manager
    }

    pub fn get_entities(&self) -> EntityIterator<'_, 1> {
        let prefix = ObjectVertex::build_prefix_prefix(Prefix::VertexEntity.prefix_id());
        let snapshot_iterator = self.snapshot.iterate_range(
            KeyRange::new_within(prefix, Prefix::VertexEntity.fixed_width_keys())
        );
        EntityIterator::new(snapshot_iterator)
    }

    pub fn get_relations(&self) -> RelationIterator<'_, 1> {
        let prefix = ObjectVertex::build_prefix_prefix(Prefix::VertexRelation.prefix_id());
        let snapshot_iterator = self.snapshot.iterate_range(KeyRange::new_within(
            prefix, Prefix::VertexRelation.fixed_width_keys())
        );
        RelationIterator::new(snapshot_iterator)
    }

    pub fn get_attributes(&self) -> AttributeIterator<'_, Snapshot, 1, 2> {
        let start = AttributeVertex::build_prefix_prefix(Prefix::ATTRIBUTE_MIN);
        let end = AttributeVertex::build_prefix_prefix(Prefix::ATTRIBUTE_MAX);
        let attribute_iterator = self.snapshot.iterate_range(KeyRange::new_inclusive(start, end));

        let has_reverse_start = ThingEdgeHasReverse::prefix_from_prefix(Prefix::ATTRIBUTE_MIN);
        let has_reverse_end = ThingEdgeHasReverse::prefix_from_prefix(Prefix::ATTRIBUTE_MAX);
        let has_reverse_iterator = self.snapshot.iterate_range(KeyRange::new_inclusive(
            has_reverse_start, has_reverse_end,
        ));
        AttributeIterator::new(attribute_iterator, has_reverse_iterator, self.type_manager())
    }

    pub fn get_attributes_in(
        &self,
        attribute_type: AttributeType<'_>,
    ) -> Result<AttributeIterator<'_, Snapshot, 3, 4>, ConceptReadError> {
        Ok(attribute_type
            .get_value_type(self.type_manager.as_ref())?
            .map(|value_type| {
                let attribute_value_type_prefix = AttributeVertex::value_type_to_prefix_type(value_type);
                let prefix = AttributeVertex::build_prefix_type(
                    attribute_value_type_prefix, attribute_type.vertex().type_id_(),
                );
                let attribute_iterator = self.snapshot.iterate_range(
                    KeyRange::new_within(prefix, attribute_value_type_prefix.fixed_width_keys())
                );

                let has_reverse_prefix = ThingEdgeHasReverse::prefix_from_type(
                    attribute_value_type_prefix, attribute_type.vertex().type_id_(),
                );
                let has_reverse_iterator = self.snapshot.iterate_range(
                    KeyRange::new_within(has_reverse_prefix, ThingEdgeHasReverse::FIXED_WIDTH_ENCODING)
                );
                AttributeIterator::new(attribute_iterator, has_reverse_iterator, self.type_manager())
            })
            .unwrap_or_else(AttributeIterator::new_empty))
    }

    pub(crate) fn get_attribute_value(&self, attribute: &Attribute<'_>) -> Result<Value<'static>, ConceptReadError> {
        match attribute.value_type() {
            ValueType::Boolean => {
                todo!()
            }
            ValueType::Long => {
                let attribute_id = LongAttributeID::new(attribute.vertex().attribute_id().unwrap_bytes_8());
                Ok(Value::Long(Long::new(attribute_id.bytes()).as_i64()))
            }
            ValueType::Double => {
                todo!()
            }
            ValueType::String => {
                let attribute_id = StringAttributeID::new(attribute.vertex().attribute_id().unwrap_bytes_17());
                if attribute_id.is_inline() {
                    Ok(Value::String(Cow::Owned(
                        String::from(attribute_id.get_inline_string_bytes().as_str()),
                    )))
                } else {
                    Ok(self
                        .snapshot
                        .get_mapped(attribute.vertex().as_storage_key().as_reference(), |bytes| {
                            Value::String(Cow::Owned(
                                String::from(StringBytes::new(Bytes::<1>::Reference(bytes)).as_str()),
                            ))
                        })
                        .map_err(|error| ConceptReadError::SnapshotGet { source: error })?
                        .unwrap())
                }
            }
        }
    }

    pub(crate) fn has_attribute<'a>(
        &self,
        owner: impl ObjectAPI<'a>,
        attribute_type: AttributeType<'static>,
        value: Value<'_>,
    ) -> Result<bool, ConceptReadError> {
        let attribute_vertex = self.encode_expected_attribute(attribute_type, value);
        let has = ThingEdgeHas::build(owner.vertex(), attribute_vertex);
        let has_exists = self.snapshot.get_mapped(has.into_storage_key().as_reference(), |value| true)
            .map_err(|err| ConceptReadError::SnapshotGet { source: err })?
            .unwrap_or(false);
        Ok(has_exists)
    }

    fn encode_expected_attribute(&self, attribute_type: AttributeType<'static>, value: Value<'_>) -> AttributeVertex<'_> {
        let value_type = attribute_type.get_value_type(self.type_manager()).unwrap().unwrap();
        debug_assert_eq!(value.value_type(), value_type);
        let attribute_id = match value {
            Value::Boolean(bool) => {
                todo!()
            }
            Value::Long(long) => {
                self.vertex_generator.compute_attribute_id_long(Long::build(long)).as_attribute_id()
            }
            Value::Double(double) => {
                todo!()
            }
            Value::String(string) => {
                let string_bytes = StringBytes::<256>::build_ref(string.as_ref().as_ref());
                self.vertex_generator
                    .compute_attribute_id_string(attribute_type.vertex().type_id_(), string_bytes, self.snapshot.as_ref())
                    .as_attribute_id()
            }
        };
        AttributeVertex::build(value_type, attribute_type.vertex().type_id_(), attribute_id)
    }

    pub(crate) fn get_has<'this, 'a>(
        &'this self,
        owner: impl ObjectAPI<'a>,
    ) -> HasAttributeIterator<'this, { ThingEdgeHas::LENGTH_PREFIX_FROM_OBJECT }> {
        let prefix = ThingEdgeHas::prefix_from_object(owner.into_vertex());
        HasAttributeIterator::new(self.snapshot.iterate_range(
            KeyRange::new_within(prefix, ThingEdgeHas::FIXED_WIDTH_ENCODING)
        ))
    }

    pub(crate) fn get_has_type<'this, 'a>(
        &'this self,
        owner: impl ObjectAPI<'a>,
        attribute_type: AttributeType<'static>,
    ) -> HasAttributeIterator<'this, { ThingEdgeHas::LENGTH_PREFIX_FROM_OBJECT_TO_TYPE }> {
        let prefix = ThingEdgeHas::prefix_from_object_to_type(
            owner.into_vertex(), attribute_type.get_value_type(self.type_manager()).unwrap().unwrap(),
            attribute_type.into_vertex(),
        );
        HasAttributeIterator::new(self.snapshot.iterate_range(
            KeyRange::new_within(prefix, ThingEdgeHas::FIXED_WIDTH_ENCODING)
        ))
    }

    pub(crate) fn get_owners<'this, 'a>(
        &'this self,
        attribute: Attribute<'a>,
    ) -> AttributeOwnerIterator<'this, { ThingEdgeHasReverse::LENGTH_BOUND_PREFIX_FROM }> {
        let prefix = ThingEdgeHasReverse::prefix_from_attribute(attribute.into_vertex());
        AttributeOwnerIterator::new(self.snapshot.iterate_range(
            KeyRange::new_within(prefix, ThingEdgeHasReverse::FIXED_WIDTH_ENCODING)
        ))
    }

    pub(crate) fn has_owners<'a>(&self, attribute: Attribute<'a>, buffered_only: bool) -> bool {
        let prefix = ThingEdgeHasReverse::prefix_from_attribute(attribute.into_vertex());
        self.snapshot.any_in_range(
            KeyRange::new_within(prefix, ThingEdgeHasReverse::FIXED_WIDTH_ENCODING), buffered_only,
        )
    }

    pub(crate) fn get_relations_roles<'this, 'a>(
        &'this self,
        player: impl ObjectAPI<'a>,
    ) -> RelationRoleIterator<'this, { ThingEdgeRolePlayer::LENGTH_PREFIX_FROM }> {
        let prefix = ThingEdgeRolePlayer::prefix_reverse_from_player(player.into_vertex());
        RelationRoleIterator::new(self.snapshot.iterate_range(
            KeyRange::new_within(prefix, ThingEdgeRolePlayer::FIXED_WIDTH_ENCODING),
        ))
    }

    pub(crate) fn has_role_players<'a>(
        &self,
        relation: Relation<'a>,
        buffered_only: bool, // FIXME use enums
    ) -> bool {
        let prefix = ThingEdgeRolePlayer::prefix_from_relation(relation.into_vertex());
        self.snapshot.any_in_range(
            KeyRange::new_within(prefix, ThingEdgeRolePlayer::FIXED_WIDTH_ENCODING), buffered_only,
        )
    }

    pub(crate) fn get_role_players<'a>(
        &self,
        relation: impl ObjectAPI<'a>,
    ) -> RolePlayerIterator<'_, { ThingEdgeHas::LENGTH_PREFIX_FROM_OBJECT }> {
        let prefix = ThingEdgeRolePlayer::prefix_from_relation(relation.into_vertex());
        RolePlayerIterator::new(self.snapshot.iterate_range(
            KeyRange::new_within(prefix, ThingEdgeRolePlayer::FIXED_WIDTH_ENCODING)
        ))
    }

    pub(crate) fn get_indexed_players(
        &self,
        from: Object<'_>,
    ) -> IndexedPlayersIterator<'_, { ThingEdgeRelationIndex::LENGTH_PREFIX_FROM }> {
        let prefix = ThingEdgeRelationIndex::prefix_from(from.vertex());
        IndexedPlayersIterator::new(self.snapshot.iterate_range(KeyRange::new_within(
            prefix, ThingEdgeRelationIndex::FIXED_WIDTH_ENCODING
        )))
    }

    pub(crate) fn get_status(&self, key: StorageKey<'_, BUFFER_KEY_INLINE>) -> ConceptStatus {
        self.snapshot
            .get_buffered_write_mapped(key.as_reference(), |write| match write {
                Write::Insert { .. } => ConceptStatus::Inserted,
                Write::Put { .. } => ConceptStatus::Put,
                Write::Delete => ConceptStatus::Deleted,
            })
            .unwrap_or_else(|| {
                debug_assert!(self.snapshot.get::<BUFFER_KEY_INLINE>(key.as_reference()).unwrap().is_some());
                ConceptStatus::Persisted
            })
    }
}

impl<'txn, Snapshot: WritableSnapshot> ThingManager<Snapshot> {
    pub(crate) fn relation_compound_update_mutex(&self) -> &Mutex<()> {
        &self.relation_lock
    }

    pub(crate) fn lock_existing<'a>(&self, object: impl ObjectAPI<'a>) {
        self.snapshot.unmodifiable_lock_add(object.into_vertex().as_storage_key().into_owned_array())
    }

    pub fn finalise(self) -> Result<(), Vec<ConceptWriteError>> {
        self.cleanup_relations().map_err(|err| Vec::from([err]))?;
        self.cleanup_attributes().map_err(|err| Vec::from([err]))?;
        let thing_errors = self.thing_errors();
        match thing_errors {
            Ok(errors) => if errors.is_empty() { Ok(()) } else { Err(errors) }
            Err(error) => Err(Vec::from([ConceptWriteError::ConceptRead { source: error }]))
        }
    }

    fn cleanup_relations(&self) -> Result<(), ConceptWriteError> {
        let mut any_deleted = true;
        while any_deleted {
            any_deleted = false;
            for (key, _) in self
                .snapshot
                .iterate_writes_range(KeyRange::new_within(
                    ThingEdgeRolePlayer::prefix().into_byte_array_or_ref(),
                    ThingEdgeRolePlayer::FIXED_WIDTH_ENCODING
                ))
                .filter(|(_, write)| matches!(write, Write::Delete))
            {
                let edge = ThingEdgeRolePlayer::new(Bytes::Reference(ByteReference::from(key.byte_array())));
                let relation = Relation::new(edge.to());
                if !relation.has_players(self) {
                    relation.delete(self)?;
                    any_deleted = true;
                }
            }
        }
        Ok(())
    }

    fn cleanup_attributes(&self) -> Result<(), ConceptWriteError> {
        for (key, _) in self
            .snapshot
            .iterate_writes_range(KeyRange::new_within(
                ThingEdgeHas::prefix().into_byte_array_or_ref(),
                ThingEdgeHas::FIXED_WIDTH_ENCODING
            ))
            .filter(|(_, write)| matches!(write, Write::Delete))
        {
            let edge = ThingEdgeHas::new(Bytes::Reference(ByteReference::from(key.byte_array())));
            let attribute = Attribute::new(edge.to());
            let is_independent = attribute.type_().is_independent(self.type_manager())
                .map_err(|err| ConceptWriteError::ConceptRead { source: err })?;
            if !is_independent && !attribute.has_owners(self) {
                attribute.delete(self)?;
            }
        }
        Ok(())
    }

    fn thing_errors(&self) -> Result<Vec<ConceptWriteError>, ConceptReadError> {
        let mut errors = Vec::new();
        let mut relations_validated = HashSet::new();
        for (key, _) in self
            .snapshot
            .iterate_writes_range(KeyRange::new_within(
                ThingEdgeRolePlayer::prefix().into_byte_array_or_ref(),
                ThingEdgeRolePlayer::FIXED_WIDTH_ENCODING,
            ))
        {
            let edge = ThingEdgeRolePlayer::new(Bytes::Reference(ByteReference::from(key.byte_array())));
            let relation = Relation::new(edge.from());
            if !relations_validated.contains(&relation) {
                errors.extend(relation.errors(self)?);
                relations_validated.insert(relation.into_owned());
            }
        }
        Ok(errors)
    }

    pub fn create_entity(&self, entity_type: EntityType<'static>) -> Result<Entity<'_>, ConceptWriteError> {
        Ok(Entity::new(self.vertex_generator.create_entity(entity_type.vertex().type_id_(), self.snapshot.as_ref())))
    }

    pub fn create_relation(&self, relation_type: RelationType<'static>) -> Result<Relation<'_>, ConceptWriteError> {
        Ok(Relation::new(self.vertex_generator.create_relation(relation_type.vertex().type_id_(), self.snapshot.as_ref())))
    }

    pub fn create_attribute(
        &self,
        attribute_type: AttributeType<'static>,
        value: Value<'_>,
    ) -> Result<Attribute<'_>, ConceptWriteError> {
        let value_type = attribute_type.get_value_type(self.type_manager.as_ref())?;
        if Some(value.value_type()) == value_type {
            let vertex = match value {
                Value::Boolean(_bool) => {
                    todo!()
                }
                Value::Long(long) => {
                    let encoded_long = Long::build(long);
                    self.vertex_generator.create_attribute_long(
                        attribute_type.vertex().type_id_(),
                        encoded_long,
                        self.snapshot.as_ref(),
                    )
                }
                Value::Double(_double) => {
                    todo!()
                }
                Value::String(string) => {
                    let encoded_string: StringBytes<'_, BUFFER_KEY_INLINE> = StringBytes::build_ref(&string);
                    self.vertex_generator.create_attribute_string(
                        attribute_type.vertex().type_id_(),
                        encoded_string,
                        self.snapshot.as_ref(),
                    )
                }
            };
            Ok(Attribute::new(vertex))
        } else {
            Err(ConceptWriteError::ValueTypeMismatch { expected: value_type, provided: value.value_type() })
        }
    }

    pub(crate) fn delete_entity(&self, entity: Entity<'_>) {
        let key = entity.into_vertex().into_storage_key().into_owned_array();
        self.snapshot.unmodifiable_lock_remove(&key);
        self.snapshot.delete(key)
    }

    pub(crate) fn delete_relation(&self, relation: Relation<'_>) {
        let key = relation.into_vertex().into_storage_key().into_owned_array();
        self.snapshot.unmodifiable_lock_remove(&key);
        self.snapshot.delete(key)
    }

    pub(crate) fn delete_attribute(&self, attribute: Attribute<'_>) {
        let key = attribute.into_vertex().into_storage_key().into_owned_array();
        self.snapshot.delete(key);
    }

    pub(crate) fn set_has<'a>(&self, owner: impl ObjectAPI<'a>, attribute: Attribute<'_>) {
        // TODO: handle duplicates
        // note: we always re-put the attribute. TODO: optimise knowing when the attribute pre-exists.
        self.snapshot.put(attribute.vertex().as_storage_key().into_owned_array());
        owner.set_modified(self);
        let has = ThingEdgeHas::build(owner.vertex(), attribute.vertex());
        self.snapshot.put_val(has.into_storage_key().into_owned_array(), encode_value_u64(1));
        let has_reverse = ThingEdgeHasReverse::build(attribute.into_vertex(), owner.into_vertex());
        self.snapshot.put_val(has_reverse.into_storage_key().into_owned_array(), encode_value_u64(1));
    }

    pub(crate) fn delete_has<'a>(&self, owner: impl ObjectAPI<'a>, attribute: Attribute<'_>) {
        owner.set_modified(self);
        let has = ThingEdgeHas::build(owner.vertex(), attribute.vertex());
        self.snapshot.delete(has.into_storage_key().into_owned_array());
        let has_reverse = ThingEdgeHasReverse::build(attribute.into_vertex(), owner.into_vertex());
        self.snapshot.delete(has_reverse.into_storage_key().into_owned_array());
    }

    pub(crate) fn increment_has<'a>(&self, owner: impl ObjectAPI<'a>, attribute: Attribute<'_>) {
        todo!()
    }

    pub(crate) fn decrement_has<'a>(&self, owner: impl ObjectAPI<'a>, attribute: Attribute<'a>, decrement_count: u64) {
        todo!()
    }

    pub fn set_role_player<'a>(&self, relation: Relation<'_>, player: impl ObjectAPI<'a>, role_type: RoleType<'_>) {
        let role_player =
            ThingEdgeRolePlayer::build_role_player(relation.vertex(), player.vertex(), role_type.clone().into_vertex());
        let count: u64 = 1;
        self.snapshot.put_val(role_player.into_storage_key().into_owned_array(), encode_value_u64(count));
        let role_player_reverse = ThingEdgeRolePlayer::build_role_player_reverse(
            player.into_vertex(),
            relation.into_vertex(),
            role_type.into_vertex(),
        );
        // must be idempotent, so no lock required -- cannot fail
        self.snapshot.put_val(role_player_reverse.into_storage_key().into_owned_array(), encode_value_u64(count));
    }

    pub fn delete_role_player<'a>(&self, relation: Relation<'_>, player: impl ObjectAPI<'a>, role_type: RoleType<'_>) {
        let role_player = ThingEdgeRolePlayer::build_role_player(
            relation.vertex(), player.vertex(), role_type.clone().into_vertex(),
        );
        self.snapshot.delete(role_player.into_storage_key().into_owned_array());
        let role_player_reverse = ThingEdgeRolePlayer::build_role_player_reverse(
            player.into_vertex(),
            relation.into_vertex(),
            role_type.into_vertex(),
        );
        self.snapshot.delete(role_player_reverse.into_storage_key().into_owned_array());
    }

    ///
    /// Add a player to a relation that supports duplicates
    /// Caller must provide a lock that prevents race conditions on the player counts on the relation
    ///
    pub(crate) fn increment_role_player<'a>(&self,
                                            relation: Relation<'_>,
                                            player: impl ObjectAPI<'a>,
                                            role_type: RoleType<'_>,
                                            _update_guard: &MutexGuard<'_, ()>,
    ) -> u64 {
        let role_player =
            ThingEdgeRolePlayer::build_role_player(relation.vertex(), player.vertex(), role_type.clone().into_vertex());
        let role_player_reverse =
            ThingEdgeRolePlayer::build_role_player_reverse(player.vertex(), relation.vertex(), role_type.into_vertex());

        let mut count = 0;
        let rp_count = self.snapshot
            .get_mapped(role_player.as_storage_key().as_reference(), |val| {
                decode_value_u64(val)
            }).unwrap();
        let rp_reverse_count = self.snapshot
            .get_mapped(role_player_reverse.as_storage_key().as_reference(), |val| {
                decode_value_u64(val)
            }).unwrap();
        debug_assert_eq!(&rp_count, &rp_reverse_count);

        count = rp_count.unwrap_or(0) + 1;
        let reverse_count = rp_reverse_count.unwrap_or(0) + 1;
        self.snapshot.put_val(role_player.as_storage_key().into_owned_array(), encode_value_u64(count));
        self.snapshot
            .put_val(role_player_reverse.as_storage_key().into_owned_array(), encode_value_u64(reverse_count));

        // must lock to fail concurrent transactions updating the same counters
        self.snapshot.exclusive_lock_add(role_player.into_storage_key());
        count
    }

    ///
    /// Remove a player to a relation that supports duplicates
    /// Caller must provide a lock that prevents race conditions on the player counts on the relation
    ///
    pub(crate) fn decrement_role_player<'a>(&self,
                                            relation: Relation<'_>,
                                            player: impl ObjectAPI<'a>,
                                            role_type: RoleType<'_>,
                                            decrement_count: u64,
                                            _update_guard: &MutexGuard<'_, ()>,
    ) -> u64 {
        let role_player = ThingEdgeRolePlayer::build_role_player(
            relation.vertex(), player.vertex(), role_type.clone().into_vertex(),
        );
        let role_player_reverse = ThingEdgeRolePlayer::build_role_player_reverse(
            player.vertex(), relation.vertex(), role_type.into_vertex(),
        );

        let mut count = 0;
        let rp_count = self.snapshot
            .get_mapped(role_player.as_storage_key().as_reference(), |val| {
                decode_value_u64(val)
            }).unwrap();
        let rp_reverse_count = self.snapshot
            .get_mapped(role_player_reverse.as_storage_key().as_reference(), |val| {
                decode_value_u64(val)
            }).unwrap();
        debug_assert_eq!(&rp_count, &rp_reverse_count);

        count = rp_count.unwrap() - decrement_count;
        debug_assert!(count >= 0);
        let reverse_count = rp_reverse_count.unwrap() - decrement_count;
        debug_assert!(reverse_count >= 0);
        if count == 0 {
            self.snapshot.delete(role_player.as_storage_key().into_owned_array());
            self.snapshot.delete(role_player_reverse.as_storage_key().into_owned_array());
        } else {
            self.snapshot.put_val(role_player.as_storage_key().into_owned_array(), encode_value_u64(count));
            self.snapshot
                .put_val(role_player_reverse.as_storage_key().into_owned_array(), encode_value_u64(reverse_count));
        }

        // must lock to fail concurrent transactions updating the same counters
        self.snapshot.exclusive_lock_add(role_player.into_storage_key());
        count
    }

    ///
    /// Clean up all parts of a relation index to do with a specific role player.
    /// Caller must provide a lock that guarantees the relation's player is removed before and atomically
    ///
    pub(crate) fn relation_index_player_deleted(
        &self,
        relation: Relation<'_>,
        player: Object<'_>,
        role_type: RoleType<'_>,
        _update_guard: &MutexGuard<'_, ()>,
    ) {
        let mut players = relation.get_players(self);
        let mut role_player = players.next().transpose().unwrap();
        while let Some((rp, count)) = role_player {
            debug_assert_eq!(count, 1);
            let index = ThingEdgeRelationIndex::build(
                player.vertex(),
                rp.player().vertex(),
                relation.vertex(),
                role_type.vertex().type_id_(),
                rp.role_type().vertex().type_id_(),
            );
            self.snapshot.delete(index.as_storage_key().into_owned_array());
            let index_reverse = ThingEdgeRelationIndex::build(
                rp.player().vertex(),
                player.vertex(),
                relation.vertex(),
                rp.role_type().vertex().type_id_(),
                role_type.vertex().type_id_(),
            );
            self.snapshot.delete(index_reverse.as_storage_key().into_owned_array());
            role_player = players.next().transpose().unwrap();
        }
    }

    ///
    /// For N duplicate role players, the self-edges are available N-1 times.
    /// For N duplicate player 1, and M duplicate player 2 - from N to M has M index repetitions, while M to N has N index repetitions
    ///
    pub(crate) fn relation_index_player_regenerate(
        &self,
        relation: Relation<'_>,
        player: Object<'_>,
        role_type: RoleType<'_>,
        total_player_count: u64,
        _update_guard: &MutexGuard<'_, ()>,
    ) {
        debug_assert_ne!(total_player_count, 0);
        let mut players = relation.get_players(self);
        let mut role_player = players.next().transpose().unwrap();
        while let Some((rp, count)) = role_player.as_ref() {
            let is_same_rp = rp.player() == player && rp.role_type() == role_type;
            if is_same_rp && total_player_count > 1 {
                let repetitions = total_player_count - 1;
                let index = ThingEdgeRelationIndex::build(
                    player.vertex(),
                    player.vertex(),
                    relation.vertex(),
                    role_type.vertex().type_id_(),
                    role_type.vertex().type_id_(),
                );
                self.snapshot.put_val(index.as_storage_key().into_owned_array(), encode_value_u64(repetitions));
            } else if !is_same_rp {
                let rp_repetitions = *count;
                let index = ThingEdgeRelationIndex::build(
                    player.vertex(),
                    rp.player().vertex(),
                    relation.vertex(),
                    role_type.vertex().type_id_(),
                    rp.role_type().vertex().type_id_(),
                );
                self.snapshot.put_val(index.as_storage_key().into_owned_array(), encode_value_u64(rp_repetitions));
                let player_repetitions = total_player_count;
                let index_reverse = ThingEdgeRelationIndex::build(
                    rp.player().vertex(),
                    player.vertex(),
                    relation.vertex(),
                    rp.role_type().vertex().type_id_(),
                    role_type.vertex().type_id_(),
                );
                self.snapshot.put_val(index_reverse.as_storage_key().into_owned_array(), encode_value_u64(player_repetitions));
            }
            role_player = players.next().transpose().unwrap();
        }
    }
}