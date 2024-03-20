/*
 *  Copyright (C) 2023 Vaticle
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as
 *  published by the Free Software Foundation, either version 3 of the
 *  License, or (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::{collections::HashSet, ops::Deref};

use encoding::{
    graph::type_::vertex::TypeVertex,
    value::{label::Label, value_type::ValueType},
};
use primitive::maybe_owns::MaybeOwns;

use crate::{
    type_::{
        attribute_type::{AttributeType, AttributeTypeAnnotation},
        entity_type::{EntityType, EntityTypeAnnotation},
        owns::Owns,
        relation_type::{RelationType, RelationTypeAnnotation},
        type_manager::TypeManager,
    },
    ConceptAPI,
};
use crate::type_::relates::Relates;
use crate::type_::role_type::{RoleType, RoleTypeAnnotation};

pub mod annotation;
pub mod attribute_type;
pub mod entity_type;
pub mod object_type;
pub mod owns;
mod plays;
mod relates;
pub mod relation_type;
pub mod type_cache;
pub mod type_manager;
pub mod role_type;

pub trait TypeAPI<'a>: ConceptAPI<'a> + Sized + Clone {
    fn vertex<'this>(&'this self) -> &'this TypeVertex<'a>;

    fn into_vertex(self) -> TypeVertex<'a>;
}

pub trait EntityTypeAPI<'a>: TypeAPI<'a> {
    fn is_root(&self, type_manager: &TypeManager) -> bool {
        type_manager.get_entity_type_is_root(self.clone().into_owned())
    }

    fn get_label<'m>(&self, type_manager: &'m TypeManager) -> MaybeOwns<'m, Label<'static>> {
        type_manager.get_entity_type_label(self.clone().into_owned())
    }

    fn set_label(&self, type_manager: &TypeManager, label: &Label) {
        // TODO: setLabel should fail is setting label on Root type
        type_manager.set_storage_label(self.vertex().clone().into_owned(), label);
    }

    // TODO: not so pretty to return EntityType directly, but is a win on efficiency. However, should reconsider the trait's necessity.
    fn get_supertype(&self, type_manager: &TypeManager) -> Option<EntityType<'static>> {
        type_manager.get_entity_type_supertype(self.clone().into_owned())
    }

    fn set_supertype(&self, type_manager: &TypeManager, supertype: impl EntityTypeAPI<'static>) {
        type_manager.set_storage_supertype(self.vertex().clone().into_owned(), supertype.vertex().clone().into_owned())
    }

    // TODO: not so pretty to return EntityType directly, but is a win on efficiency. However, should reconsider the trait's necessity.
    fn get_supertypes<'m>(&self, type_manager: &'m TypeManager) -> MaybeOwns<'m, Vec<EntityType<'static>>> {
        type_manager.get_entity_type_supertypes(self.clone().into_owned())
    }

    // fn get_subtypes(&self) -> MaybeOwns<'m, Vec<EntityType<'static>>>;

    fn get_annotations<'m>(&self, type_manager: &'m TypeManager) -> MaybeOwns<'m, HashSet<EntityTypeAnnotation>> {
        type_manager.get_entity_type_annotations(self.clone().into_owned())
    }

    fn set_annotation(&self, type_manager: &TypeManager, annotation: EntityTypeAnnotation) {
        match annotation {
            EntityTypeAnnotation::Abstract(_) => {
                type_manager.set_storage_annotation_abstract(self.vertex().clone().into_owned())
            }
        }
    }

    fn delete_annotation(&self, type_manager: &TypeManager, annotation: EntityTypeAnnotation) {
        match annotation {
            EntityTypeAnnotation::Abstract(_) => {
                type_manager.delete_storage_annotation_abstract(self.vertex().clone().into_owned())
            }
        }
    }

    // TODO: not so pretty to return EntityType directly, but is a win on efficiency. However, should reconsider the trait's necessity.
    fn into_owned(self) -> EntityType<'static>;
}

pub trait RelationTypeAPI<'a>: TypeAPI<'a> {
    fn is_root(&self, type_manager: &TypeManager) -> bool {
        type_manager.get_relation_type_is_root(self.clone().into_owned())
    }

    fn get_label<'m>(&self, type_manager: &'m TypeManager) -> MaybeOwns<'m, Label<'static>> {
        type_manager.get_relation_type_label(self.clone().into_owned())
    }

    fn set_label(&self, type_manager: &TypeManager, label: &Label) {
        // TODO: setLabel should fail is setting label on Root type
        type_manager.set_storage_label(self.vertex().clone().into_owned(), label);
    }

    fn get_supertype(&self, type_manager: &TypeManager) -> Option<RelationType<'static>> {
        type_manager.get_relation_type_supertype(self.clone().into_owned())
    }

    fn set_supertype(&self, type_manager: &TypeManager, supertype: impl RelationTypeAPI<'static>) {
        type_manager.set_storage_supertype(self.vertex().clone().into_owned(), supertype.vertex().clone().into_owned())
    }

    // TODO: not so pretty to return Type directly, but is a win on efficiency. However, should reconsider the trait's necessity.
    fn get_supertypes<'m>(&self, type_manager: &'m TypeManager) -> MaybeOwns<'m, Vec<RelationType<'static>>> {
        type_manager.get_relation_type_supertypes(self.clone().into_owned())
    }

    // fn get_subtypes(&self) -> MaybeOwns<'m, Vec<RelationType<'static>>>;

    fn get_annotations<'m>(&self, type_manager: &'m TypeManager) -> MaybeOwns<'m, HashSet<RelationTypeAnnotation>> {
        type_manager.get_relation_type_annotations(self.clone().into_owned())
    }

    fn set_annotation(&self, type_manager: &TypeManager, annotation: RelationTypeAnnotation) {
        match annotation {
            RelationTypeAnnotation::Abstract(_) => {
                type_manager.set_storage_annotation_abstract(self.vertex().clone().into_owned())
            }
        }
    }

    fn delete_annotation(&self, type_manager: &TypeManager, annotation: RelationTypeAnnotation) {
        match annotation {
            RelationTypeAnnotation::Abstract(_) => {
                type_manager.delete_storage_annotation_abstract(self.vertex().clone().into_owned())
            }
        }
    }

    fn get_role(&self, type_manager: &TypeManager, name: &str) -> Option<RoleType<'static>> {
        let label = Label::build_scoped(name, self.get_label(type_manager).name().decode());
        type_manager.get_role_type(&label)
    }

    fn create_relates(&self, type_manager: &TypeManager, name: &str) -> Relates<'static> {
        let label = Label::build_scoped(name, self.get_label(type_manager).name().decode());
        let role_type = type_manager.create_role_type(&label, self.clone().into_owned(), false);
        self.get_relates_role(type_manager, name).unwrap()
    }

    fn delete_relates(&self, type_manager: &TypeManager, role_type: RoleType<'static>) {
        type_manager.delete_storage_relates(self.vertex().clone().into_owned(), role_type.into_vertex());
    }

    fn get_relates<'m>(&self, type_manager: &'m TypeManager) -> MaybeOwns<'m, HashSet<Relates<'static>>> {
        type_manager.get_relation_type_relates(self.clone().into_owned())
    }

    fn get_relates_role(&self, type_manager: &TypeManager, name: &str) -> Option<Relates<'static>> {
        self.get_role(type_manager, name).map(|role_type|
            Relates::new(self.clone().into_owned(), role_type)
        )
    }

    fn has_relates_role(&self, type_manager: &TypeManager, name: &str) -> bool {
        self.get_relates_role(type_manager, name).is_some()
    }

    fn into_owned(self) -> RelationType<'static>;
}

pub trait RoleTypeAPI<'a>: TypeAPI<'a> {
    fn is_root(&self, type_manager: &TypeManager) -> bool {
        type_manager.get_role_type_is_root(self.clone().into_owned())
    }

    fn get_label<'m>(&self, type_manager: &'m TypeManager) -> MaybeOwns<'m, Label<'static>> {
        type_manager.get_role_type_label(self.clone().into_owned())
    }

    fn set_name(&self, type_manager: &TypeManager, name: &str) {
        // // TODO: setLabel should fail is setting label on Root type
        // type_manager.set_storage_label(self.vertex().clone().into_owned(), label);


        todo!()
    }

    fn get_supertype(&self, type_manager: &TypeManager) -> Option<RoleType<'static>> {
        type_manager.get_role_type_supertype(self.clone().into_owned())
    }

    fn set_supertype(&self, type_manager: &TypeManager, supertype: impl RoleTypeAPI<'static>) {
        type_manager.set_storage_supertype(self.vertex().clone().into_owned(), supertype.vertex().clone().into_owned())
    }

    // TODO: not so pretty to return Type directly, but is a win on efficiency. However, should reconsider the trait's necessity.
    fn get_supertypes<'m>(&self, type_manager: &'m TypeManager) -> MaybeOwns<'m, Vec<RoleType<'static>>> {
        type_manager.get_role_type_supertypes(self.clone().into_owned())
    }

    // fn get_subtypes(&self) -> MaybeOwns<'m, Vec<RoleType<'static>>>;

    fn get_annotations<'m>(&self, type_manager: &'m TypeManager) -> MaybeOwns<'m, HashSet<RoleTypeAnnotation>> {
        type_manager.get_role_type_annotations(self.clone().into_owned())
    }

    fn set_annotation(&self, type_manager: &TypeManager, annotation: RoleTypeAnnotation) {
        match annotation {
            RoleTypeAnnotation::Abstract(_) => {
                type_manager.set_storage_annotation_abstract(self.vertex().clone().into_owned())
            }
        }
    }

    fn delete_annotation(&self, type_manager: &TypeManager, annotation: RoleTypeAnnotation) {
        match annotation {
            RoleTypeAnnotation::Abstract(_) => {
                type_manager.delete_storage_annotation_abstract(self.vertex().clone().into_owned())
            }
        }
    }

    fn get_relates(&self, type_manager: &TypeManager) -> Relates<'static> {
        todo!()
    }

    fn into_owned(self) -> RoleType<'static>;
}

pub trait AttributeTypeAPI<'a>: TypeAPI<'a> {
    fn is_root(&self, type_manager: &TypeManager) -> bool {
        type_manager.get_attribute_type_is_root(self.clone().into_owned())
    }

    fn set_value_type(&self, type_manager: &TypeManager, value_type: ValueType) {
        type_manager.set_storage_value_type(self.vertex().clone().into_owned(), value_type)
    }

    fn get_value_type(&self, type_manager: &TypeManager) -> Option<ValueType> {
        type_manager.get_attribute_type_value_type(self.clone().into_owned())
    }

    fn get_label<'m>(&self, type_manager: &'m TypeManager) -> MaybeOwns<'m, Label<'static>> {
        type_manager.get_attribute_type_label(self.clone().into_owned())
    }

    fn set_label(&self, type_manager: &TypeManager, label: &Label) {
        // TODO: setLabel should fail is setting label on Root type
        type_manager.set_storage_label(self.vertex().clone().into_owned(), label);
    }

    fn get_supertype(&self, type_manager: &TypeManager) -> Option<AttributeType<'static>> {
        type_manager.get_attribute_type_supertype(self.clone().into_owned())
    }

    fn set_supertype(&self, type_manager: &TypeManager, supertype: impl AttributeTypeAPI<'static>) {
        type_manager.set_storage_supertype(self.vertex().clone().into_owned(), supertype.vertex().clone().into_owned())
    }

    // TODO: not so pretty to return Type directly, but is a win on efficiency. However, should reconsider the trait's necessity.
    fn get_supertypes<'m>(&self, type_manager: &'m TypeManager) -> MaybeOwns<'m, Vec<AttributeType<'static>>> {
        type_manager.get_attribute_type_supertypes(self.clone().into_owned())
    }

    // fn get_subtypes(&self) -> MaybeOwns<'m, Vec<AttributeType<'static>>>;

    fn get_annotations<'m>(&self, type_manager: &'m TypeManager) -> MaybeOwns<'m, HashSet<AttributeTypeAnnotation>> {
        type_manager.get_attribute_type_annotations(self.clone().into_owned())
    }

    fn set_annotation(&self, type_manager: &TypeManager, annotation: AttributeTypeAnnotation) {
        match annotation {
            AttributeTypeAnnotation::Abstract(_) => {
                type_manager.set_storage_annotation_abstract(self.vertex().clone().into_owned())
            }
        }
    }

    fn delete_annotation(&self, type_manager: &TypeManager, annotation: AttributeTypeAnnotation) {
        match annotation {
            AttributeTypeAnnotation::Abstract(_) => {
                type_manager.delete_storage_annotation_abstract(self.vertex().clone().into_owned())
            }
        }
    }

    fn into_owned(self) -> AttributeType<'static>;
}

pub trait OwnerAPI<'a>: TypeAPI<'a> {
    fn set_owns(&self, type_manager: &TypeManager, attribute_type: AttributeType<'static>) -> Owns<'static> {
        type_manager.set_storage_owns(self.vertex().clone().into_owned(), attribute_type.clone().into_vertex());
        self.get_owns_attribute(type_manager, attribute_type).unwrap()
    }

    fn delete_owns(&self, type_manager: &TypeManager, attribute_type: AttributeType<'static>) {
        type_manager.delete_storage_owns(self.vertex().clone().into_owned(), attribute_type.clone().into_vertex());
    }

    fn _construct_owns(&self, attribute_type: AttributeType<'static>) -> Owns<'static>;

    fn get_owns<'m>(&self, type_manager: &'m TypeManager) -> MaybeOwns<'m, HashSet<Owns<'static>>>;

    fn get_owns_attribute(
        &self,
        type_manager: &TypeManager,
        attribute_type: AttributeType<'static>,
    ) -> Option<Owns<'static>> {
        let expected_owns = self._construct_owns(attribute_type);
        if self.get_owns(type_manager).deref().contains(&expected_owns) {
            Some(expected_owns)
        } else {
            None
        }
    }

    fn has_owns_attribute(&self, type_manager: &TypeManager, attribute_type: AttributeType<'static>) -> bool {
        self.get_owns_attribute(type_manager, attribute_type).is_some()
    }
}

trait OwnedAPI<'a>: AttributeTypeAPI<'a> {
    fn get_owns<'m>(&self, _type_manager: &'m TypeManager) -> MaybeOwns<'m, HashSet<Owns<'static>>> {
        todo!()
    }

    fn get_owns_owners(&self) {
        // return iterator of Owns
        todo!()
    }
}

trait PlayerAPI<'a>: TypeAPI<'a> {
    // fn set_plays(&self, role_type: &RoleType) -> Plays;

    fn get_plays(&self) {
        // return iterator of Plays
        todo!()
    }

    fn get_plays_played(&self) {
        // return iterator of played types
        todo!()
    }

    // fn has_plays_played(&self, role_type: &RoleType);
}

trait PlayedAPI<'a>: TypeAPI<'a> {
    fn get_plays(&self) {
        // return iterator of Plays
        todo!()
    }

    fn get_plays_players(&self) {
        // return iterator of player types
        todo!()
    }
}