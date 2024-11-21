/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Display, Formatter};

use compiler::ExecutorVariable;
use ir::pattern::constraint::FunctionCallBinding;

pub(crate) struct FunctionCallBindingIteratorExecutor {
    function_call_binding: FunctionCallBinding<ExecutorVariable>,
}

impl Display for FunctionCallBindingIteratorExecutor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[{}]", &self.function_call_binding)
    }
}
