/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use concept::type_::{Ordering, OwnerAPI, TypeAPI};
use cucumber::gherkin::Step;
use itertools::Itertools;
use macro_rules_attribute::apply;

use super::thing_type::get_as_object_type;
use crate::{
    generic_step, params,
    transaction_context::{with_read_tx, with_schema_tx},
    util, Context,
};

#[apply(generic_step)]
#[step(expr = "{root_label}\\({type_label}\\) set owns: {type_label}")]
pub async fn set_owns(
    context: &mut Context,
    root_label: params::RootLabel,
    type_label: params::Label,
    attribute_type_label: params::Label,
) {
    let object_type = get_as_object_type(context, root_label.to_typedb(), &type_label);
    with_schema_tx!(context, |tx| {
        let attr_type =
            tx.type_manager.get_attribute_type(&tx.snapshot, &attribute_type_label.to_typedb()).unwrap().unwrap();
        object_type.set_owns(&mut tx.snapshot, &tx.type_manager, attr_type, Ordering::Unordered).unwrap();
    });
}

#[apply(generic_step)]
#[step(expr = "{root_label}\\({type_label}\\) set owns: {type_label}[]")]
pub async fn set_owns_ordered(
    context: &mut Context,
    root_label: params::RootLabel,
    type_label: params::Label,
    attribute_type_label: params::Label,
) {
    let object_type = get_as_object_type(context, root_label.to_typedb(), &type_label);
    with_schema_tx!(context, |tx| {
        let attr_type =
            tx.type_manager.get_attribute_type(&tx.snapshot, &attribute_type_label.to_typedb()).unwrap().unwrap();
        object_type.set_owns(&mut tx.snapshot, &tx.type_manager, attr_type, Ordering::Ordered).unwrap();
    });
}

#[apply(generic_step)]
#[step(expr = "{root_label}\\({type_label}\\) unset owns: {type_label}")]
pub async fn unset_owns(
    context: &mut Context,
    root_label: params::RootLabel,
    type_label: params::Label,
    attribute_type_label: params::Label,
) {
    let object_type = get_as_object_type(context, root_label.to_typedb(), &type_label);
    with_schema_tx!(context, |tx| {
        let attr_type =
            tx.type_manager.get_attribute_type(&tx.snapshot, &attribute_type_label.to_typedb()).unwrap().unwrap();
        object_type.delete_owns(&mut tx.snapshot, &tx.type_manager, attr_type).unwrap();
    });
}

#[apply(generic_step)]
#[step(expr = "{root_label}\\({type_label}\\) get owns: {type_label}; set override: {type_label}{may_error}")]
pub async fn get_owns_set_override(
    context: &mut Context,
    root_label: params::RootLabel,
    type_label: params::Label,
    attr_type_label: params::Label,
    overridden_type_label: params::Label,
    may_error: params::MayError,
) {
    let owner = get_as_object_type(context, root_label.to_typedb(), &type_label);
    with_schema_tx!(context, |tx| {
        let attr_type =
            tx.type_manager.get_attribute_type(&tx.snapshot, &attr_type_label.to_typedb()).unwrap().unwrap();
        let owns = owner.get_owns_attribute(&tx.snapshot, &tx.type_manager, attr_type).unwrap().unwrap();

        let owner_supertype = owner.get_supertype(&tx.snapshot, &tx.type_manager).unwrap().unwrap();
        let overridden_attr_type =
            tx.type_manager.get_attribute_type(&tx.snapshot, &overridden_type_label.to_typedb()).unwrap().unwrap();

        let overridden_owns_opt = owner_supertype
            .get_owns_attribute_transitive(&tx.snapshot, &tx.type_manager, overridden_attr_type)
            .unwrap(); // This may also error
        if let Some(overridden_owns) = overridden_owns_opt {
            let res = owns.set_override(&mut tx.snapshot, &tx.type_manager, overridden_owns);
            may_error.check(&res);
        } else {
            assert!(may_error.expects_error()); // We error by not finding the type to override
        }
    });
}

#[apply(generic_step)]
#[step(expr = "{root_label}\\({type_label}\\) get owns: {type_label}, set annotation: {annotation}{may_error}")]
pub async fn get_owns_set_annotation(
    context: &mut Context,
    root_label: params::RootLabel,
    type_label: params::Label,
    attr_type_label: params::Label,
    annotation: params::Annotation,
    may_error: params::MayError,
) {
    let object_type = get_as_object_type(context, root_label.to_typedb(), &type_label);
    with_schema_tx!(context, |tx| {
        let attr_type =
            tx.type_manager.get_attribute_type(&tx.snapshot, &attr_type_label.to_typedb()).unwrap().unwrap();
        let owns = object_type.get_owns_attribute(&tx.snapshot, &tx.type_manager, attr_type).unwrap().unwrap();
        let res = owns.set_annotation(&mut tx.snapshot, &tx.type_manager, annotation.into_typedb().into());
        may_error.check(&res);
    });
}

#[apply(generic_step)]
#[step(
    expr = "{root_label}\\({type_label}\\) get owns: {type_label}; get annotations {contains_or_doesnt}: {annotation}"
)]
pub async fn get_owns_get_annotations_contains(
    context: &mut Context,
    root_label: params::RootLabel,
    type_label: params::Label,
    attr_type_label: params::Label,
    contains_or_doesnt: params::ContainsOrDoesnt,
    annotation: params::Annotation,
) {
    let object_type = get_as_object_type(context, root_label.to_typedb(), &type_label);
    with_read_tx!(context, |tx| {
        let attr_type =
            tx.type_manager.get_attribute_type(&tx.snapshot, &attr_type_label.to_typedb()).unwrap().unwrap();
        let owns =
            object_type.get_owns_attribute_transitive(&tx.snapshot, &tx.type_manager, attr_type).unwrap().unwrap();
        let actual_contains = owns
            .get_effective_annotations(&tx.snapshot, &tx.type_manager)
            .unwrap()
            .contains_key(&annotation.into_typedb().into());
        assert_eq!(contains_or_doesnt.expected_contains(), actual_contains);
    });
}

#[apply(generic_step)]
#[step(expr = "{root_label}\\({type_label}\\) get owns {contains_or_doesnt}:")]
pub async fn get_owns_contain(
    context: &mut Context,
    root_label: params::RootLabel,
    type_label: params::Label,
    contains: params::ContainsOrDoesnt,
    step: &Step,
) {
    let expected_labels = util::iter_table(step).map(|str| str.to_owned()).collect_vec();
    let object_type = get_as_object_type(context, root_label.to_typedb(), &type_label);
    with_read_tx!(context, |tx| {
        let actual_labels = object_type
            .get_owns_transitive(&tx.snapshot, &tx.type_manager)
            .unwrap()
            .iter()
            .map(|(_attribute, owns)| {
                owns.attribute().get_label(&tx.snapshot, &tx.type_manager).unwrap().scoped_name().as_str().to_owned()
            })
            .collect_vec();
        contains.check(&expected_labels, &actual_labels);
    });
}

#[apply(generic_step)]
#[step(expr = "{root_label}\\({type_label}\\) get declared owns {contains_or_doesnt}:")]
pub async fn get_declared_owns_contain(
    context: &mut Context,
    root_label: params::RootLabel,
    type_label: params::Label,
    contains: params::ContainsOrDoesnt,
    step: &Step,
) {
    let expected_labels = util::iter_table(step).map(|str| str.to_owned()).collect_vec();
    let object_type = get_as_object_type(context, root_label.to_typedb(), &type_label);
    with_read_tx!(context, |tx| {
        let actual_labels = object_type
            .get_owns(&tx.snapshot, &tx.type_manager)
            .unwrap()
            .iter()
            .map(|owns| {
                owns.attribute().get_label(&tx.snapshot, &tx.type_manager).unwrap().scoped_name().as_str().to_owned()
            })
            .collect_vec();
        contains.check(&expected_labels, &actual_labels);
    });
}

#[apply(generic_step)]
#[step(expr = "{root_label}\\({type_label}\\) get owns overridden\\({type_label}\\) {exists_or_doesnt}")]
pub async fn get_owns_overridden_exists(
    context: &mut Context,
    root_label: params::RootLabel,
    type_label: params::Label,
    attr_type_label: params::Label,
    exists: params::ExistsOrDoesnt,
) {
    let object_type = get_as_object_type(context, root_label.to_typedb(), &type_label);
    with_read_tx!(context, |tx| {
        let attr_type =
            tx.type_manager.get_attribute_type(&tx.snapshot, &attr_type_label.to_typedb()).unwrap().unwrap();
        let owns =
            object_type.get_owns_attribute_transitive(&tx.snapshot, &tx.type_manager, attr_type).unwrap().unwrap();
        let overridden_owns_opt = owns.get_override(&tx.snapshot, &tx.type_manager).unwrap();
        exists.check(
            &overridden_owns_opt,
            &format!("override for {} owns {}", type_label.to_typedb(), attr_type_label.to_typedb()),
        );
    });
}

#[apply(generic_step)]
#[step(expr = "{root_label}\\({type_label}\\) get owns overridden\\({type_label}\\) get label: {type_label}")]
pub async fn get_owns_overridden_get_label(
    context: &mut Context,
    root_label: params::RootLabel,
    type_label: params::Label,
    attr_type_label: params::Label,
    expected_overridden: params::Label,
) {
    let owner = get_as_object_type(context, root_label.to_typedb(), &type_label);
    with_read_tx!(context, |tx| {
        let attr_type =
            tx.type_manager.get_attribute_type(&tx.snapshot, &attr_type_label.to_typedb()).unwrap().unwrap();
        let owns = owner.get_owns_attribute_transitive(&tx.snapshot, &tx.type_manager, attr_type).unwrap().unwrap();
        let overridden_owns_opt = owns.get_override(&tx.snapshot, &tx.type_manager).unwrap();
        let overridden_owns = overridden_owns_opt.as_ref().unwrap();
        let actual_type_label = overridden_owns
            .attribute()
            .get_label(&tx.snapshot, &tx.type_manager)
            .unwrap()
            .scoped_name()
            .as_str()
            .to_owned();
        assert_eq!(expected_overridden.to_typedb().scoped_name().as_str().to_owned(), actual_type_label);
    });
}