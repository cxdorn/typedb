/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use answer::variable::Variable;
use error::unimplemented_feature;
use structural_equality::StructuralEquality;

use crate::{
    pattern::{
        conjunction::{Conjunction, ConjunctionBuilder},
        Scope, ScopeId,
    },
    pipeline::block::BlockBuilderContext,
};

#[derive(Debug, Clone)]
pub struct Optional {
    conjunction: Conjunction,
}

impl Optional {
    pub fn new(scope_id: ScopeId) -> Self {
        Self { conjunction: Conjunction::new(scope_id) }
    }

    pub(super) fn new_builder<'cx, 'reg>(
        context: &'cx mut BlockBuilderContext<'reg>,
        optional: &'cx mut Optional,
    ) -> ConjunctionBuilder<'cx, 'reg> {
        ConjunctionBuilder::new(context, &mut optional.conjunction)
    }

    pub fn conjunction(&self) -> &Conjunction {
        &self.conjunction
    }

    pub fn conjunction_mut(&mut self) -> &mut Conjunction {
        &mut self.conjunction
    }
}

impl Scope for Optional {
    fn scope_id(&self) -> ScopeId {
        self.conjunction.scope_id()
    }
}

impl StructuralEquality for Optional {
    fn hash(&self) -> u64 {
        self.conjunction.hash()
    }

    fn equals(&self, other: &Self) -> bool {
        self.conjunction.equals(&other.conjunction)
    }
}

impl fmt::Display for Optional {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        error::todo_display_for_error!(f, self)
    }
}
