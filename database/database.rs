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

use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use concept::type_manager::TypeManager;
use encoding::initialise_storage;
use encoding::type_::id_generator::TypeIIDGenerator;
use encoding::thing::id_generator::ThingIIDGenerator;
use storage::snapshot2::Snapshot;
use storage::MVCCStorage;
use crate::error::DatabaseError;
use crate::error::DatabaseErrorKind::{FailedToCreateStorage, FailedToSetupStorage};
use crate::transaction::{TransactionRead, TransactionWrite};

pub struct Database {
    name: Rc<str>,
    path: PathBuf,
    storage: MVCCStorage,
    type_iid_generator: TypeIIDGenerator,
    thing_iid_generator: ThingIIDGenerator,
}

impl Database {
    pub fn new(path: &PathBuf, database_name: Rc<str>) -> Result<Database, DatabaseError> {
        let database_path = path.with_extension(String::from(database_name.as_ref()));
        fs::create_dir(database_path.as_path());
        let mut storage = MVCCStorage::new(database_name.clone(), path)
            .map_err(|storage_error| DatabaseError {
                database_name: database_name.to_string(),
                kind: FailedToCreateStorage(storage_error),
            })?;

        initialise_storage(&mut storage).map_err(|storage_error| DatabaseError {
            database_name: database_name.to_string(),
            kind: FailedToSetupStorage(storage_error),
        })?;
        let type_iid_generator = TypeIIDGenerator::new();
        let thing_iid_generator = ThingIIDGenerator::new();
        TypeManager::initialise_types(&mut storage, &type_iid_generator);

        let database = Database {
            name: database_name.clone(),
            path: database_path,
            storage: storage,
            type_iid_generator: type_iid_generator,
            thing_iid_generator: thing_iid_generator,
        };
        Ok(database)
    }

    pub fn transaction_read(&self) -> TransactionRead {
        let mut snapshot: Rc<Snapshot<'_>> = Rc::new(Snapshot::Read(self.storage.snapshot_read()));
        let type_manager = TypeManager::new(snapshot.clone(), &self.type_iid_generator);
        TransactionRead {
            snapshot: snapshot,
            type_manager: type_manager,
        }
    }

    fn transaction_write(&self) -> TransactionWrite {
        let mut snapshot: Rc<Snapshot<'_>> = Rc::new(Snapshot::Write(self.storage.snapshot_write()));
        let type_manager = TypeManager::new(snapshot.clone(), &self.type_iid_generator);
        TransactionWrite {
            snapshot: snapshot,
            type_manager: type_manager,
        }
    }
}
