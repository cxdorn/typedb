/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
    sync::Arc,
};
use durability::SequenceNumber;
use encoding::{
    graph::Typed, value::{label::Label, value_type::ValueType}, Prefixed
};
use storage::{snapshot::ReadableSnapshot, MVCCStorage, ReadSnapshotOpenError};

use crate::type_::{attribute_type::AttributeType, entity_type::EntityType, owns::{Owns, OwnsAnnotation}, plays::Plays, relates::Relates, relation_type::RelationType, role_type::RoleType, type_manager::ReadableType, Ordering, TypeAPI, OwnerAPI, PlayerAPI};
use crate::type_::type_cache::kind_cache::{AttributeTypeCache, EntityTypeCache, RelationTypeCache, RoleTypeCache, OwnsCache, CommonTypeCache, OwnerPlayerCache};
use crate::type_::type_cache::selection;
use crate::type_::type_cache::selection::{HasOwnerPlayerCache, HasCommonTypeCache, CacheGetter};
use crate::type_::type_manager::KindAPI;


// TODO: could/should we slab allocate the schema cache?
pub struct TypeCache {
    open_sequence_number: SequenceNumber,

    // Types that are borrowable and returned from the cache
    entity_types: Box<[Option<EntityTypeCache>]>,
    relation_types: Box<[Option<RelationTypeCache>]>,
    role_types: Box<[Option<RoleTypeCache>]>,
    attribute_types: Box<[Option<AttributeTypeCache>]>,

    owns: HashMap<Owns<'static>, OwnsCache>,

    entity_types_index_label: HashMap<Label<'static>, EntityType<'static>>,
    relation_types_index_label: HashMap<Label<'static>, RelationType<'static>>,
    role_types_index_label: HashMap<Label<'static>, RoleType<'static>>,
    attribute_types_index_label: HashMap<Label<'static>, AttributeType<'static>>,
}


selection::impl_cache_getter!(EntityTypeCache, EntityType, entity_types);
selection::impl_cache_getter!(AttributeTypeCache, AttributeType, attribute_types);
selection::impl_cache_getter!(RelationTypeCache, RelationType, relation_types);
selection::impl_cache_getter!(RoleTypeCache, RoleType, role_types);

selection::impl_has_common_type_cache!(EntityTypeCache, EntityType<'static>);
selection::impl_has_common_type_cache!(AttributeTypeCache, AttributeType<'static>);
selection::impl_has_common_type_cache!(RelationTypeCache, RelationType<'static>);
selection::impl_has_common_type_cache!(RoleTypeCache, RoleType<'static>);

selection::impl_has_owner_player_cache!(EntityTypeCache, EntityType<'static>);
selection::impl_has_owner_player_cache!(RelationTypeCache, RelationType<'static>);

impl TypeCache {
    // If creation becomes slow, We should restore pre-fetching of the schema
    //  with a single pass on disk (as it was in 1f339733feaf4542e47ff604462f107d2ade1f1a)
    pub fn new<D>(
        storage: Arc<MVCCStorage<D>>,
        open_sequence_number: SequenceNumber,
    ) -> Result<Self, TypeCacheCreateError> {
        use TypeCacheCreateError::SnapshotOpen;
        // note: since we will parse out many heterogenous properties/edges from the schema, we will scan once into a vector,
        //       then go through it again to pull out the type information.

        let snapshot =
            storage.open_snapshot_read_at(open_sequence_number).map_err(|error| SnapshotOpen { source: error })?;

        let entity_type_caches = EntityTypeCache::create(&snapshot);
        let relation_type_caches = RelationTypeCache::create(&snapshot);
        let role_type_caches = RoleTypeCache::create(&snapshot);
        let attribute_type_caches = AttributeTypeCache::create(&snapshot);

        let entity_types_index_label = Self::build_label_to_type_index(&entity_type_caches);
        let relation_types_index_label = Self::build_label_to_type_index(&relation_type_caches);
        let role_types_index_label = Self::build_label_to_type_index(&role_type_caches);
        let attribute_types_index_label = Self::build_label_to_type_index(&attribute_type_caches);

        Ok(TypeCache {
            open_sequence_number,
            entity_types: entity_type_caches,
            relation_types: relation_type_caches,
            role_types: role_type_caches,
            attribute_types: attribute_type_caches,
            owns: OwnsCache::create(&snapshot),

            entity_types_index_label,
            relation_types_index_label,
            role_types_index_label,
            attribute_types_index_label,
        })
    }

    fn build_label_to_type_index<T: KindAPI<'static>, CACHE: HasCommonTypeCache<T>>(
        type_cache_array: &Box<[Option<CACHE>]>,
    ) -> HashMap<Label<'static>, T> {
        type_cache_array
            .iter()
            .filter_map(|entry| {
                entry
                    .as_ref()
                    .map(|cache| (cache.common_type_cache().label.clone(), cache.common_type_cache().type_.clone()))
            })
            .collect()
    }

    pub(crate) fn get_entity_type(&self, label: &Label<'_>) -> Option<EntityType<'static>> {
        self.entity_types_index_label.get(label).cloned()
    }

    pub(crate) fn get_relation_type(&self, label: &Label<'_>) -> Option<RelationType<'static>> {
        self.relation_types_index_label.get(label).cloned()
    }

    pub(crate) fn get_role_type(&self, label: &Label<'_>) -> Option<RoleType<'static>> {
        self.role_types_index_label.get(label).cloned()
    }

    pub(crate) fn get_attribute_type(&self, label: &Label<'_>) -> Option<AttributeType<'static>> {
        self.attribute_types_index_label.get(label).cloned()
    }

    pub(crate) fn get_supertype<'a, 'this, T, CACHE>(&'this self, type_: T) -> Option<T::SelfStatic>
        where T: KindAPI<'a> + CacheGetter<CacheType=CACHE>,
              CACHE: HasCommonTypeCache<T::SelfStatic> + 'this
    {
        // TODO: Why does this not return &Option<EntityType<'static>> ?
        Some(T::get_cache(self, type_).common_type_cache().supertype.as_ref()?.clone())
    }

    pub(crate) fn get_supertypes<'a, 'this, T, CACHE>(&'this self, type_: T) -> &'this Vec<T::SelfStatic>
        where T: KindAPI<'a> + CacheGetter<CacheType=CACHE>,
              CACHE: HasCommonTypeCache<T::SelfStatic> + 'this
    {
        &T::get_cache(self, type_).common_type_cache().supertypes
    }
    pub(crate) fn get_subtypes<'a, 'this, T, CACHE>(&'this self, type_: T) -> &'this Vec<T::SelfStatic>
        where T: KindAPI<'a> + CacheGetter<CacheType=CACHE>,
              CACHE: HasCommonTypeCache<T::SelfStatic> + 'this
    {
        &T::get_cache(self, type_).common_type_cache().subtypes_declared
    }

    pub(crate) fn get_subtypes_transitive<'a, 'this, T, CACHE>(&'this self, type_: T) -> &'this Vec<T::SelfStatic>
        where T: KindAPI<'a> + CacheGetter<CacheType=CACHE>,
              CACHE: HasCommonTypeCache<T::SelfStatic> + 'this
    {
        &T::get_cache(self, type_).common_type_cache().subtypes_transitive
    }

    pub(crate) fn get_label<'a, 'this, T, CACHE>(&'this self, type_: T) -> &'this Label<'static>
        where T: KindAPI<'a> + CacheGetter<CacheType=CACHE>,
              CACHE : HasCommonTypeCache<T::SelfStatic> + 'this
    {
        &T::get_cache(self, type_).common_type_cache().label
    }

    pub(crate) fn is_root<'a, 'this, T, CACHE>(&'this self, type_: T) -> bool
        where T: KindAPI<'a> + CacheGetter<CacheType=CACHE>,
              CACHE: HasCommonTypeCache<T::SelfStatic> + 'this
    {
        T::get_cache(self, type_).common_type_cache().is_root
    }

    pub(crate) fn get_annotations<'a, 'this, T, CACHE>(&'this self, type_: T) -> &HashSet<<<T as KindAPI<'a>>::SelfStatic as KindAPI<'static>>::AnnotationType>
        where T: KindAPI<'a> + CacheGetter<CacheType=CACHE>,
              CACHE: HasCommonTypeCache<T::SelfStatic> + 'this
    {
        &T::get_cache(self, type_).common_type_cache().annotations_declared
    }

    pub(crate) fn get_owns<'a, 'this, T, CACHE>(&'this self, type_: T) -> &HashSet<Owns<'static>>
        where T:  OwnerAPI<'static> + PlayerAPI<'static> + CacheGetter<CacheType=CACHE>,
              CACHE: HasOwnerPlayerCache + 'this
    {
        &T::get_cache(self, type_).owner_player_cache().owns_declared
    }

    pub(crate) fn get_role_type_ordering(&self, role_type: RoleType<'static>) -> Ordering {
        RoleType::get_cache(&self, role_type).ordering
    }

    pub(crate) fn get_relation_type_relates(&self, relation_type: RelationType<'static>) -> &HashSet<Relates<'static>> {
        &RelationType::get_cache(self, relation_type).relates_declared
        // &Self::get_relation_type_cache(&self.relation_types, relation_type.into_vertex()).unwrap().relates_declared
    }

    pub(crate) fn get_plays<'a, 'this, T, CACHE>(&'this self, type_: T) -> &HashSet<Plays<'static>>
        where T:  OwnerAPI<'static> + PlayerAPI<'static> + CacheGetter<CacheType=CACHE>,
              CACHE: HasOwnerPlayerCache + 'this
    {
        &T::get_cache(self, type_).owner_player_cache().plays_declared
    }

    pub(crate) fn get_attribute_type_value_type(&self, attribute_type: AttributeType<'static>) -> Option<ValueType> {
        AttributeType::get_cache(&self, attribute_type).value_type
    }

    pub(crate) fn get_owns_annotations<'c>(&'c self, owns: Owns<'c>) -> &'c HashSet<OwnsAnnotation> {
        &self.owns.get(&owns).unwrap().annotations_declared
    }

    pub(crate) fn get_owns_ordering<'c>(&'c self, owns: Owns<'c>) -> Ordering {
        self.owns.get(&owns).unwrap().ordering
    }
}

#[derive(Debug)]
pub enum TypeCacheCreateError {
    SnapshotOpen { source: ReadSnapshotOpenError },
}

impl fmt::Display for TypeCacheCreateError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SnapshotOpen { .. } => todo!(),
        }
    }
}

impl Error for TypeCacheCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SnapshotOpen { source } => Some(source),
        }
    }
}