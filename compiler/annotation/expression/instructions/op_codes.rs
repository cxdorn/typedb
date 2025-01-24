/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// TODO: Rewrite so we generate the dispatcher macro along with the enum. SEe https://cprohm.de/blog/rust-macros/
#[derive(Debug, Clone)]
pub enum ExpressionOpCode {
    // Basics
    LoadConstant,
    LoadVariable,
    ListConstructor,
    ListIndex,
    ListIndexRange,

    // Casts
    // TODO: We can't cast arguments for functions of arity > 2. It may require rewriting compilation.
    CastUnaryIntegerToDouble,
    CastLeftIntegerToDouble,
    CastRightIntegerToDouble,
    CastUnaryIntegerToDecimal,

    CastLeftIntegerToDecimal,
    CastRightIntegerToDecimal,

    CastUnaryDecimalToDouble,
    CastLeftDecimalToDouble,
    CastRightDecimalToDouble,

    // Operators
    OpIntegerAddInteger,
    OpDoubleAddDouble,
    OpIntegerMultiplyInteger,

    OpIntegerSubtractInteger,
    OpIntegerDivideInteger,
    OpIntegerModuloInteger,
    OpIntegerPowerInteger,

    OpDoubleSubtractDouble,
    OpDoubleMultiplyDouble,
    OpDoubleDivideDouble,
    OpDoubleModuloDouble,
    OpDoublePowerDouble,

    OpDecimalAddDecimal,
    OpDecimalSubtractDecimal,
    OpDecimalMultiplyDecimal,

    // BuiltIns, maybe by domain?
    MathAbsInteger,
    MathAbsDouble,
    MathRemainderInteger,
    MathRoundDouble,
    MathCeilDouble,
    MathFloorDouble,
}
