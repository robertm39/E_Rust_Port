use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::terms::functypes::FunCode;
use crate::terms::simpletypes::{type_is_predicate, Type};
use crate::terms::typebanks::TypeBank;
use std::collections::BTreeMap;
use std::ops::{BitOr, BitOrAssign};

pub const DEFAULT_SIGNATURE_SIZE: usize = 20;
pub const DEFAULT_SIGNATURE_GROW: usize = 2;
pub const SIG_FEATURE_ARITY_LIMIT: i32 = 6;

pub const SIG_TRUE_CODE: FunCode = 1;
pub const SIG_FALSE_CODE: FunCode = 2;
pub const SIG_NIL_CODE: FunCode = 3;
pub const SIG_CONS_CODE: FunCode = 4;
pub const SIG_PHONY_APP_CODE: FunCode = 17;
pub const SIG_NAMED_LAMBDA_CODE: FunCode = SIG_PHONY_APP_CODE + 1;
pub const SIG_DB_LAMBDA_CODE: FunCode = SIG_NAMED_LAMBDA_CODE + 1;
pub const SIG_ITE_CODE: FunCode = SIG_DB_LAMBDA_CODE + 1;
pub const SIG_LET_CODE: FunCode = SIG_ITE_CODE + 1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FunctionProperties(u64);

impl FunctionProperties {
    pub const IGNORE_PROPS: Self = Self(0);
    pub const TYPE_FIXED: Self = Self(1);
    pub const TYPE_POLY: Self = Self(2);
    pub const FOF_OP: Self = Self(4);
    pub const SPECIAL: Self = Self(8);
    pub const ASSOCIATIVE: Self = Self(16);
    pub const COMMUTATIVE: Self = Self(32);
    pub const IS_AC: Self = Self(Self::ASSOCIATIVE.0 | Self::COMMUTATIVE.0);
    pub const INTERPRETED: Self = Self(64);
    pub const IS_INTEGER: Self = Self(128);
    pub const IS_RATIONAL: Self = Self(256);
    pub const IS_FLOAT: Self = Self(512);
    pub const IS_OBJECT: Self = Self(1024);
    pub const DISTINCT_PROP: Self =
        Self(Self::IS_OBJECT.0 | Self::IS_INTEGER.0 | Self::IS_RATIONAL.0 | Self::IS_FLOAT.0);
    pub const OP_FLAG: Self = Self(2048);
    pub const CL_SPLIT_DEF: Self = Self(4096);
    pub const PSEUDO_PRED: Self = Self(8192);
    pub const TYPED_APPLICATION: Self = Self(Self::PSEUDO_PRED.0 * 2);
    pub const IS_INJ_DEF_SKOLEM: Self = Self(Self::TYPED_APPLICATION.0 * 2);
    pub const SKOLEM_SYMBOL: Self = Self(Self::IS_INJ_DEF_SKOLEM.0 * 2);
    pub const DEF_PRED: Self = Self(Self::SKOLEM_SYMBOL.0 * 2);
    pub const DEF_FUN: Self = Self(Self::DEF_PRED.0 * 2);

    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn contains_all(self, properties: Self) -> bool {
        (self.0 & properties.0) == properties.0
    }

    #[must_use]
    pub const fn intersects(self, properties: Self) -> bool {
        (self.0 & properties.0) != 0
    }

    pub fn insert(&mut self, properties: Self) {
        self.0 |= properties.0;
    }

    pub fn remove(&mut self, properties: Self) {
        self.0 &= !properties.0;
    }
}

impl BitOr for FunctionProperties {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for FunctionProperties {
    fn bitor_assign(&mut self, rhs: Self) {
        self.insert(rhs);
    }
}

pub const FP_IGNORE_PROPS: FunctionProperties = FunctionProperties::IGNORE_PROPS;
pub const FP_TYPE_FIXED: FunctionProperties = FunctionProperties::TYPE_FIXED;
pub const FP_TYPE_POLY: FunctionProperties = FunctionProperties::TYPE_POLY;
pub const FP_FOF_OP: FunctionProperties = FunctionProperties::FOF_OP;
pub const FP_SPECIAL: FunctionProperties = FunctionProperties::SPECIAL;
pub const FP_ASSOCIATIVE: FunctionProperties = FunctionProperties::ASSOCIATIVE;
pub const FP_COMMUTATIVE: FunctionProperties = FunctionProperties::COMMUTATIVE;
pub const FP_IS_AC: FunctionProperties = FunctionProperties::IS_AC;
pub const FP_INTERPRETED: FunctionProperties = FunctionProperties::INTERPRETED;
pub const FP_IS_INTEGER: FunctionProperties = FunctionProperties::IS_INTEGER;
pub const FP_IS_RATIONAL: FunctionProperties = FunctionProperties::IS_RATIONAL;
pub const FP_IS_FLOAT: FunctionProperties = FunctionProperties::IS_FLOAT;
pub const FP_IS_OBJECT: FunctionProperties = FunctionProperties::IS_OBJECT;
pub const FP_DISTINCT_PROP: FunctionProperties = FunctionProperties::DISTINCT_PROP;
pub const FP_OP_FLAG: FunctionProperties = FunctionProperties::OP_FLAG;
pub const FP_CL_SPLIT_DEF: FunctionProperties = FunctionProperties::CL_SPLIT_DEF;
pub const FP_PSEUDO_PRED: FunctionProperties = FunctionProperties::PSEUDO_PRED;
pub const FP_TYPED_APPLICATION: FunctionProperties = FunctionProperties::TYPED_APPLICATION;
pub const FP_IS_INJ_DEF_SKOLEM: FunctionProperties = FunctionProperties::IS_INJ_DEF_SKOLEM;
pub const FP_SKOLEM_SYMBOL: FunctionProperties = FunctionProperties::SKOLEM_SYMBOL;
pub const FP_DEF_PRED: FunctionProperties = FunctionProperties::DEF_PRED;
pub const FP_DEF_FUN: FunctionProperties = FunctionProperties::DEF_FUN;

#[derive(Clone, Debug)]
pub struct FuncCell {
    name: String,
    pname: String,
    arity: i32,
    alpha_rank: i32,
    feature_offset: i32,
    type_: Option<Type>,
    properties: FunctionProperties,
}

impl FuncCell {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn print_name(&self) -> &str {
        &self.pname
    }

    #[must_use]
    pub const fn arity(&self) -> i32 {
        self.arity
    }

    #[must_use]
    pub const fn alpha_rank(&self) -> i32 {
        self.alpha_rank
    }

    #[must_use]
    pub const fn feature_offset(&self) -> i32 {
        self.feature_offset
    }

    #[must_use]
    pub fn type_(&self) -> Option<&Type> {
        self.type_.as_ref()
    }

    #[must_use]
    pub const fn properties(&self) -> FunctionProperties {
        self.properties
    }

    fn new(name: &str, pname: &str, arity: i32) -> Self {
        Self {
            name: name.to_owned(),
            pname: pname.to_owned(),
            arity,
            alpha_rank: 0,
            feature_offset: -1,
            type_: None,
            properties: FP_IGNORE_PROPS,
        }
    }

    fn dummy() -> Self {
        Self::new("", "", 0)
    }
}

#[derive(Clone, Debug)]
pub struct Signature {
    alpha_ranks_valid: bool,
    size: usize,
    f_count: FunCode,
    internal_symbols: FunCode,
    f_info: Vec<FuncCell>,
    f_index: BTreeMap<String, FunCode>,
    type_bank: TypeBank,
    typed_symbols: bool,
    eqn_code: FunCode,
    neqn_code: FunCode,
    cnil_code: FunCode,
    or_code: FunCode,
    answer_code: FunCode,
    distinct_code: FunCode,
    skolem_count: i64,
    newpred_count: i64,
    newdef_count: i64,
    distinct_props: FunctionProperties,
}

impl Signature {
    #[must_use]
    pub fn new(type_bank: TypeBank) -> Self {
        Self::new_with_list_support(type_bank, false)
    }

    #[must_use]
    pub fn new_with_list_support(type_bank: TypeBank, support_lists: bool) -> Self {
        let mut signature = Self {
            alpha_ranks_valid: false,
            size: DEFAULT_SIGNATURE_SIZE,
            f_count: 0,
            internal_symbols: 0,
            f_info: vec![FuncCell::dummy()],
            f_index: BTreeMap::new(),
            type_bank,
            typed_symbols: false,
            eqn_code: 0,
            neqn_code: 0,
            cnil_code: 0,
            or_code: 0,
            answer_code: 0,
            distinct_code: 0,
            skolem_count: 0,
            newpred_count: 0,
            newdef_count: 0,
            distinct_props: FP_DISTINCT_PROP,
        };

        let true_code = signature.insert_id_for_problem("$true", 0, true, ProblemType::FirstOrder);
        debug_assert_eq!(true_code, SIG_TRUE_CODE);
        signature.set_func_prop(SIG_TRUE_CODE, FP_INTERPRETED);
        signature.set_type_direct(SIG_TRUE_CODE, signature.type_bank.bool_type());

        let false_code =
            signature.insert_id_for_problem("$false", 0, true, ProblemType::FirstOrder);
        debug_assert_eq!(false_code, SIG_FALSE_CODE);
        signature.set_func_prop(SIG_FALSE_CODE, FP_INTERPRETED);
        signature.set_type_direct(SIG_FALSE_CODE, signature.type_bank.bool_type());

        if support_lists {
            let nil_code =
                signature.insert_id_for_problem("$nil", 0, true, ProblemType::FirstOrder);
            debug_assert_eq!(nil_code, SIG_NIL_CODE);
            let cons_code =
                signature.insert_id_for_problem("$cons", 2, true, ProblemType::FirstOrder);
            debug_assert_eq!(cons_code, SIG_CONS_CODE);
        }

        signature.internal_symbols = signature.f_count;
        signature
    }

    #[must_use]
    pub const fn size(&self) -> usize {
        self.size
    }

    #[must_use]
    pub const fn f_count(&self) -> FunCode {
        self.f_count
    }

    #[must_use]
    pub const fn internal_symbols(&self) -> FunCode {
        self.internal_symbols
    }

    #[must_use]
    pub const fn external_symbols(&self) -> FunCode {
        self.f_count - self.internal_symbols
    }

    #[must_use]
    pub const fn typed_symbols(&self) -> bool {
        self.typed_symbols
    }

    #[must_use]
    pub const fn distinct_props(&self) -> FunctionProperties {
        self.distinct_props
    }

    #[must_use]
    pub const fn type_bank(&self) -> &TypeBank {
        &self.type_bank
    }

    pub fn type_bank_mut(&mut self) -> &mut TypeBank {
        &mut self.type_bank
    }

    #[must_use]
    pub const fn skolem_count(&self) -> i64 {
        self.skolem_count
    }

    #[must_use]
    pub const fn newpred_count(&self) -> i64 {
        self.newpred_count
    }

    #[must_use]
    pub const fn newdef_count(&self) -> i64 {
        self.newdef_count
    }

    #[must_use]
    pub const fn eqn_code(&self) -> FunCode {
        self.eqn_code
    }

    #[must_use]
    pub const fn neqn_code(&self) -> FunCode {
        self.neqn_code
    }

    #[must_use]
    pub const fn cnil_code(&self) -> FunCode {
        self.cnil_code
    }

    #[must_use]
    pub const fn or_code(&self) -> FunCode {
        self.or_code
    }

    #[must_use]
    pub const fn answer_code(&self) -> FunCode {
        self.answer_code
    }

    #[must_use]
    pub const fn distinct_code(&self) -> FunCode {
        self.distinct_code
    }

    #[must_use]
    pub fn func(&self, f_code: FunCode) -> &FuncCell {
        let index = self.valid_index(f_code);
        &self.f_info[index]
    }

    pub fn func_mut(&mut self, f_code: FunCode) -> &mut FuncCell {
        let index = self.valid_index(f_code);
        &mut self.f_info[index]
    }

    #[must_use]
    pub fn find_f_code(&self, name: &str) -> FunCode {
        let raw_name = raw_signature_name(name);
        self.f_index.get(raw_name.as_str()).copied().unwrap_or(0)
    }

    #[must_use]
    pub fn find_arity(&self, f_code: FunCode) -> Option<i32> {
        self.valid_index_opt(f_code)
            .map(|index| self.f_info[index].arity)
    }

    #[must_use]
    pub fn find_name(&self, f_code: FunCode) -> Option<&str> {
        if f_code == 0 {
            return Some("UNNAMED_DB");
        }
        self.valid_index_opt(f_code)
            .map(|index| self.f_info[index].pname.as_str())
    }

    #[must_use]
    pub fn get_type(&self, f_code: FunCode) -> Option<&Type> {
        self.func(f_code).type_.as_ref()
    }

    pub fn declare_type(&mut self, f_code: FunCode, type_: Type) -> Result<(), Diagnostic> {
        let is_fixed = self.is_fixed_type(f_code);
        let fun = self.func_mut(f_code);
        if let Some(existing) = &fun.type_ {
            if existing != &type_ {
                if is_fixed {
                    return Err(Diagnostic::new(ErrorCode::TYPE_ERROR, "type error"));
                }
                fun.type_ = Some(type_);
            }
        } else {
            fun.type_ = Some(type_);
        }
        Ok(())
    }

    pub fn declare_final_type(&mut self, f_code: FunCode, type_: Type) -> Result<(), Diagnostic> {
        if !self.is_polymorphic(f_code) {
            self.declare_type(f_code, type_)?;
            self.fix_type(f_code);
        }
        Ok(())
    }

    pub fn fix_type(&mut self, f_code: FunCode) {
        self.set_func_prop(f_code, FP_TYPE_FIXED);
    }

    #[must_use]
    pub fn is_fixed_type(&self, f_code: FunCode) -> bool {
        self.query_prop(f_code, FP_TYPE_FIXED)
    }

    #[must_use]
    pub fn is_polymorphic(&self, f_code: FunCode) -> bool {
        self.query_prop(f_code, FP_TYPE_POLY)
    }

    pub fn set_polymorphic(&mut self, f_code: FunCode, _value: bool) {
        self.set_func_prop(f_code, FP_TYPE_POLY);
    }

    #[must_use]
    pub fn is_predicate(&self, f_code: FunCode) -> bool {
        if self.query_prop(f_code, FP_TYPE_POLY) {
            return true;
        }
        self.get_type(f_code).is_some_and(type_is_predicate)
    }

    #[must_use]
    pub fn is_function(&self, f_code: FunCode) -> bool {
        if !self.query_prop(f_code, FP_TYPE_FIXED) {
            return false;
        }
        self.get_type(f_code)
            .is_some_and(|type_| !type_is_predicate(type_))
    }

    #[must_use]
    pub fn is_fun_const(&self, f_code: FunCode) -> bool {
        self.find_arity(f_code) == Some(0) && !self.is_predicate(f_code)
    }

    pub fn set_func_prop(&mut self, f_code: FunCode, prop: FunctionProperties) {
        self.func_mut(f_code).properties.insert(prop);
    }

    pub fn del_func_prop(&mut self, f_code: FunCode, prop: FunctionProperties) {
        self.func_mut(f_code).properties.remove(prop);
    }

    #[must_use]
    pub fn query_prop(&self, f_code: FunCode, prop: FunctionProperties) -> bool {
        self.func(f_code).properties.contains_all(prop)
    }

    #[must_use]
    pub fn is_any_func_prop_set(&self, f_code: FunCode, prop: FunctionProperties) -> bool {
        self.func(f_code).properties.intersects(prop)
    }

    pub fn set_special(&mut self, f_code: FunCode, value: bool) {
        if value {
            self.set_func_prop(f_code, FP_SPECIAL);
        } else {
            self.del_func_prop(f_code, FP_SPECIAL);
        }
    }

    pub fn set_all_special(&mut self, value: bool) {
        for f_code in 1..=self.f_count {
            self.set_special(f_code, value);
        }
    }

    #[must_use]
    pub fn is_special(&self, f_code: FunCode) -> bool {
        self.query_prop(f_code, FP_SPECIAL)
    }

    pub fn get_alpha_rank(&mut self, f_code: FunCode) -> i32 {
        if !self.alpha_ranks_valid {
            self.compute_alpha_ranks();
        }
        self.func(f_code).alpha_rank
    }

    pub fn insert_id(&mut self, name: &str, arity: i32, special_id: bool) -> FunCode {
        self.insert_id_for_problem(name, arity, special_id, problem_type())
    }

    pub fn insert_id_for_problem(
        &mut self,
        name: &str,
        arity: i32,
        special_id: bool,
        problem_type: ProblemType,
    ) -> FunCode {
        let (mut raw_name, mut print_name) = split_signature_name(name);
        let mut pos = self.find_f_code(&raw_name);

        if pos != 0 && self.func(pos).arity != arity && problem_type == ProblemType::FirstOrder {
            let fixed_name = format!("{name}_ARITYFIX{arity} ");
            print_name = raw_name;
            raw_name = fixed_name;
            pos = self.find_f_code(&raw_name);
        }

        if pos != 0 {
            if special_id {
                self.set_special(pos, true);
            }
            return pos;
        }

        if usize::try_from(self.f_count).is_ok_and(|count| count == self.size - 1) {
            self.size *= DEFAULT_SIGNATURE_GROW;
        }

        self.f_count += 1;
        let code = self.f_count;
        self.f_info
            .push(FuncCell::new(&raw_name, &print_name, arity));
        self.f_index.insert(raw_name, code);
        self.set_special(code, special_id);
        self.alpha_ranks_valid = false;
        code
    }

    pub fn insert_fof_op(&mut self, name: &str, arity: i32) -> FunCode {
        let f_code = self.insert_id(name, arity, true);
        self.set_func_prop(f_code, FP_FOF_OP);
        f_code
    }

    pub fn pop_id(&mut self) -> FunCode {
        if self.f_count == 0 {
            return 0;
        }

        let result = self.f_count;
        if let Some(cell) = self.f_info.pop() {
            self.f_index.remove(&cell.name);
        }
        self.f_count -= 1;
        result
    }

    pub fn backtrack(&mut self, f_count: FunCode) -> i64 {
        let mut removed = 0;
        while self.f_count > f_count {
            removed += 1;
            self.pop_id();
        }
        removed
    }

    #[must_use]
    pub fn find_max_used_arity(&self) -> i32 {
        (1..=self.f_count)
            .filter_map(|f_code| self.find_arity(f_code))
            .max()
            .unwrap_or(0)
    }

    #[must_use]
    pub fn find_max_predicate_arity(&self) -> i32 {
        let mut result = 0;
        for f_code in self.external_f_codes() {
            if self.is_predicate(f_code) && !self.is_special(f_code) {
                if let Some(arity) = self.find_arity(f_code) {
                    result = result.max(arity);
                }
            }
        }
        result
    }

    #[must_use]
    pub fn find_min_predicate_arity(&self) -> i32 {
        let mut result = i32::MAX;
        for f_code in self.external_f_codes() {
            if self.is_predicate(f_code) && !self.is_special(f_code) {
                if let Some(arity) = self.find_arity(f_code) {
                    result = result.min(arity);
                }
            }
        }
        result
    }

    #[must_use]
    pub fn find_max_function_arity(&self) -> i32 {
        let mut result = 0;
        for f_code in self.external_f_codes() {
            if !self.is_predicate(f_code) && !self.is_special(f_code) {
                if let Some(arity) = self.find_arity(f_code) {
                    result = result.max(arity);
                }
            }
        }
        result
    }

    #[must_use]
    pub fn find_min_function_arity(&self) -> i32 {
        let mut result = i32::MAX;
        for f_code in self.external_f_codes() {
            if !self.is_predicate(f_code) && !self.is_special(f_code) {
                if let Some(arity) = self.find_arity(f_code) {
                    result = result.min(arity);
                }
            }
        }
        result
    }

    #[must_use]
    pub fn count_arity_symbols(&self, arity: i32, predicates: bool) -> i32 {
        let mut result = 0;
        for f_code in self.external_f_codes() {
            if self.is_predicate(f_code) == predicates
                && !self.is_special(f_code)
                && self.find_arity(f_code) == Some(arity)
            {
                result += 1;
            }
        }
        result
    }

    #[must_use]
    pub fn count_symbols(&self, predicates: bool) -> i32 {
        let mut result = 0;
        for f_code in self.external_f_codes() {
            if !self.is_special(f_code)
                && ((predicates && self.is_predicate(f_code))
                    || (!predicates && self.is_function(f_code)))
            {
                result += 1;
            }
        }
        result
    }

    pub fn update_feature_offset(&mut self, f_code: FunCode) {
        let feature_arity = i32::min(SIG_FEATURE_ARITY_LIMIT - 1, self.func(f_code).arity);
        let offset = if self.is_predicate(f_code) {
            feature_arity + SIG_FEATURE_ARITY_LIMIT
        } else {
            feature_arity
        };
        self.func_mut(f_code).feature_offset = offset;
    }

    pub fn get_feature_offset(&mut self, f_code: FunCode) -> i32 {
        if self.func(f_code).feature_offset == -1 {
            self.update_feature_offset(f_code);
        }
        self.func(f_code).feature_offset
    }

    pub fn get_depth_feature_offset(&mut self, f_code: FunCode) -> i32 {
        self.get_feature_offset(f_code) + 2 * SIG_FEATURE_ARITY_LIMIT
    }

    fn set_type_direct(&mut self, f_code: FunCode, type_: Type) {
        self.func_mut(f_code).type_ = Some(type_);
    }

    fn valid_index(&self, f_code: FunCode) -> usize {
        self.valid_index_opt(f_code)
            .unwrap_or_else(|| panic!("invalid function code {f_code}"))
    }

    fn valid_index_opt(&self, f_code: FunCode) -> Option<usize> {
        if !(1..=self.f_count).contains(&f_code) {
            return None;
        }
        usize::try_from(f_code)
            .ok()
            .filter(|index| *index < self.f_info.len())
    }

    fn compute_alpha_ranks(&mut self) {
        for (rank, f_code) in self.f_index.values().copied().enumerate() {
            let index = usize::try_from(f_code).unwrap_or(0);
            if let Some(cell) = self.f_info.get_mut(index) {
                cell.alpha_rank = i32::try_from(rank).unwrap_or(i32::MAX);
            }
        }
        self.alpha_ranks_valid = true;
    }

    fn external_f_codes(&self) -> impl Iterator<Item = FunCode> + '_ {
        (self.internal_symbols + 1)..=self.f_count
    }
}

fn raw_signature_name(name: &str) -> String {
    split_signature_name(name).0
}

fn split_signature_name(name: &str) -> (String, String) {
    if let Some(rest) = name.strip_prefix('\'') {
        let raw = rest.strip_suffix('\'').unwrap_or(rest).to_owned();
        (raw, name.to_owned())
    } else {
        (name.to_owned(), name.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FunctionProperties, Signature, DEFAULT_SIGNATURE_GROW, DEFAULT_SIGNATURE_SIZE,
        FP_ASSOCIATIVE, FP_CL_SPLIT_DEF, FP_COMMUTATIVE, FP_DEF_FUN, FP_DEF_PRED, FP_DISTINCT_PROP,
        FP_FOF_OP, FP_IGNORE_PROPS, FP_INTERPRETED, FP_IS_AC, FP_IS_FLOAT, FP_IS_INJ_DEF_SKOLEM,
        FP_IS_INTEGER, FP_IS_OBJECT, FP_IS_RATIONAL, FP_OP_FLAG, FP_PSEUDO_PRED, FP_SKOLEM_SYMBOL,
        FP_SPECIAL, FP_TYPED_APPLICATION, FP_TYPE_FIXED, FP_TYPE_POLY, SIG_CONS_CODE,
        SIG_DB_LAMBDA_CODE, SIG_FALSE_CODE, SIG_FEATURE_ARITY_LIMIT, SIG_ITE_CODE, SIG_LET_CODE,
        SIG_NAMED_LAMBDA_CODE, SIG_NIL_CODE, SIG_PHONY_APP_CODE, SIG_TRUE_CODE,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::simple_stuff::ProblemType;
    use crate::terms::simpletypes::{
        alloc_arrow_type, alloc_simple_sort, Type, ST_BOOL, ST_INDIVIDUALS, ST_INTEGER,
    };
    use crate::terms::typebanks::TypeBank;

    fn signature() -> Signature {
        Signature::new(TypeBank::new())
    }

    #[test]
    fn constants_match_c_header() {
        assert_eq!(DEFAULT_SIGNATURE_SIZE, 20);
        assert_eq!(DEFAULT_SIGNATURE_GROW, 2);
        assert_eq!(SIG_FEATURE_ARITY_LIMIT, 6);
        assert_eq!(SIG_TRUE_CODE, 1);
        assert_eq!(SIG_FALSE_CODE, 2);
        assert_eq!(SIG_NIL_CODE, 3);
        assert_eq!(SIG_CONS_CODE, 4);
        assert_eq!(SIG_PHONY_APP_CODE, 17);
        assert_eq!(SIG_NAMED_LAMBDA_CODE, 18);
        assert_eq!(SIG_DB_LAMBDA_CODE, 19);
        assert_eq!(SIG_ITE_CODE, 20);
        assert_eq!(SIG_LET_CODE, 21);
    }

    #[test]
    fn function_property_values_match_c_enum() {
        assert_eq!(FP_IGNORE_PROPS.bits(), 0);
        assert_eq!(FP_TYPE_FIXED.bits(), 1);
        assert_eq!(FP_TYPE_POLY.bits(), 2);
        assert_eq!(FP_FOF_OP.bits(), 4);
        assert_eq!(FP_SPECIAL.bits(), 8);
        assert_eq!(FP_ASSOCIATIVE.bits(), 16);
        assert_eq!(FP_COMMUTATIVE.bits(), 32);
        assert_eq!(FP_IS_AC.bits(), 48);
        assert_eq!(FP_INTERPRETED.bits(), 64);
        assert_eq!(FP_IS_INTEGER.bits(), 128);
        assert_eq!(FP_IS_RATIONAL.bits(), 256);
        assert_eq!(FP_IS_FLOAT.bits(), 512);
        assert_eq!(FP_IS_OBJECT.bits(), 1024);
        assert_eq!(FP_DISTINCT_PROP.bits(), 1920);
        assert_eq!(FP_OP_FLAG.bits(), 2048);
        assert_eq!(FP_CL_SPLIT_DEF.bits(), 4096);
        assert_eq!(FP_PSEUDO_PRED.bits(), 8192);
        assert_eq!(FP_TYPED_APPLICATION.bits(), 16384);
        assert_eq!(FP_IS_INJ_DEF_SKOLEM.bits(), 32768);
        assert_eq!(FP_SKOLEM_SYMBOL.bits(), 65536);
        assert_eq!(FP_DEF_PRED.bits(), 131_072);
        assert_eq!(FP_DEF_FUN.bits(), 262_144);

        let mut props = FunctionProperties::TYPE_FIXED | FunctionProperties::SPECIAL;
        assert!(props.contains_all(FP_TYPE_FIXED));
        assert!(props.intersects(FP_TYPE_FIXED | FP_INTERPRETED));
        props.remove(FP_TYPE_FIXED);
        assert!(!props.contains_all(FP_TYPE_FIXED));
    }

    #[test]
    fn allocation_inserts_true_false_and_optional_lists_like_c() {
        let sig = signature();

        assert_eq!(sig.f_count(), SIG_FALSE_CODE);
        assert_eq!(sig.internal_symbols(), SIG_FALSE_CODE);
        assert_eq!(sig.external_symbols(), 0);
        assert_eq!(sig.size(), DEFAULT_SIGNATURE_SIZE);
        assert_eq!(sig.find_f_code("$true"), SIG_TRUE_CODE);
        assert_eq!(sig.find_f_code("$false"), SIG_FALSE_CODE);
        assert_eq!(sig.find_name(0), Some("UNNAMED_DB"));
        assert_eq!(sig.find_name(SIG_TRUE_CODE), Some("$true"));
        assert_eq!(sig.find_arity(SIG_FALSE_CODE), Some(0));
        assert!(sig.query_prop(SIG_TRUE_CODE, FP_INTERPRETED | FP_SPECIAL));
        assert!(sig.get_type(SIG_TRUE_CODE).is_some_and(Type::is_bool));
        assert_eq!(sig.distinct_props(), FP_DISTINCT_PROP);
        assert!(!sig.typed_symbols());
        assert_eq!(sig.skolem_count(), 0);
        assert_eq!(sig.newpred_count(), 0);
        assert_eq!(sig.newdef_count(), 0);
        assert_eq!(sig.eqn_code(), 0);
        assert_eq!(sig.neqn_code(), 0);
        assert_eq!(sig.cnil_code(), 0);
        assert_eq!(sig.or_code(), 0);
        assert_eq!(sig.answer_code(), 0);
        assert_eq!(sig.distinct_code(), 0);

        let with_lists = Signature::new_with_list_support(TypeBank::new(), true);
        assert_eq!(with_lists.f_count(), SIG_CONS_CODE);
        assert_eq!(with_lists.internal_symbols(), SIG_CONS_CODE);
        assert_eq!(with_lists.find_f_code("$nil"), SIG_NIL_CODE);
        assert_eq!(with_lists.find_f_code("$cons"), SIG_CONS_CODE);
    }

    #[test]
    fn insert_id_reuses_names_and_applies_first_order_arity_fix() {
        let mut sig = signature();

        let f = sig.insert_id_for_problem("f", 1, false, ProblemType::FirstOrder);
        assert_eq!(f, SIG_FALSE_CODE + 1);
        assert_eq!(sig.find_f_code("f"), f);
        assert_eq!(sig.find_arity(f), Some(1));
        assert!(!sig.is_special(f));

        assert_eq!(
            sig.insert_id_for_problem("f", 1, true, ProblemType::FirstOrder),
            f
        );
        assert!(sig.is_special(f));

        let fixed = sig.insert_id_for_problem("f", 2, false, ProblemType::FirstOrder);
        assert_ne!(fixed, f);
        assert_eq!(sig.find_f_code("f_ARITYFIX2 "), fixed);
        assert_eq!(sig.find_name(fixed), Some("f"));
        assert_eq!(sig.find_arity(fixed), Some(2));

        let ho_reuse = sig.insert_id_for_problem("f", 3, false, ProblemType::HigherOrder);
        assert_eq!(ho_reuse, f);
    }

    #[test]
    fn quoted_names_lookup_by_raw_name_but_keep_print_name() {
        let mut sig = signature();

        let quoted = sig.insert_id_for_problem("'quoted name'", 0, false, ProblemType::FirstOrder);

        assert_eq!(sig.find_f_code("quoted name"), quoted);
        assert_eq!(sig.find_f_code("'quoted name'"), quoted);
        assert_eq!(sig.func(quoted).name(), "quoted name");
        assert_eq!(sig.func(quoted).print_name(), "'quoted name'");
    }

    #[test]
    fn pop_and_backtrack_remove_last_symbols_without_shrinking_capacity() {
        let mut sig = signature();
        let f = sig.insert_id_for_problem("f", 1, false, ProblemType::FirstOrder);
        let g = sig.insert_id_for_problem("g", 2, false, ProblemType::FirstOrder);
        let h = sig.insert_id_for_problem("h", 3, false, ProblemType::FirstOrder);

        assert_eq!(sig.pop_id(), h);
        assert_eq!(sig.find_f_code("h"), 0);
        assert_eq!(sig.f_count(), g);
        assert_eq!(sig.backtrack(f), 1);
        assert_eq!(sig.find_f_code("g"), 0);
        assert_eq!(sig.find_f_code("f"), f);
        assert_eq!(sig.backtrack(0), 3);
        assert_eq!(sig.f_count(), 0);
        assert_eq!(sig.pop_id(), 0);
        assert_eq!(sig.size(), DEFAULT_SIGNATURE_SIZE);
    }

    #[test]
    fn signature_grows_by_c_multiplier_at_capacity_boundary() {
        let mut sig = signature();

        for i in 0..17 {
            sig.insert_id_for_problem(&format!("f{i}"), 0, false, ProblemType::FirstOrder);
        }
        assert_eq!(sig.f_count(), 19);
        assert_eq!(sig.size(), DEFAULT_SIGNATURE_SIZE);

        sig.insert_id_for_problem("trigger", 0, false, ProblemType::FirstOrder);
        assert_eq!(sig.f_count(), 20);
        assert_eq!(sig.size(), DEFAULT_SIGNATURE_SIZE * DEFAULT_SIGNATURE_GROW);
    }

    #[test]
    fn property_helpers_and_polymorphic_value_quirk_match_c() {
        let mut sig = signature();
        let f = sig.insert_id_for_problem("f", 0, false, ProblemType::FirstOrder);

        sig.set_func_prop(f, FP_ASSOCIATIVE | FP_COMMUTATIVE);
        assert!(sig.query_prop(f, FP_IS_AC));
        assert!(sig.is_any_func_prop_set(f, FP_ASSOCIATIVE | FP_INTERPRETED));
        sig.del_func_prop(f, FP_COMMUTATIVE);
        assert!(!sig.query_prop(f, FP_IS_AC));

        sig.set_polymorphic(f, false);
        assert!(sig.is_polymorphic(f));
        assert!(sig.is_predicate(f));
    }

    #[test]
    fn type_declarations_drive_predicate_and_function_classification() {
        let mut sig = signature();
        let f = sig.insert_id_for_problem("f", 1, false, ProblemType::FirstOrder);
        let p = sig.insert_id_for_problem("p", 1, false, ProblemType::FirstOrder);
        let int = alloc_simple_sort(ST_INTEGER);
        let ind = alloc_simple_sort(ST_INDIVIDUALS);
        let bool = alloc_simple_sort(ST_BOOL);
        let f_type = alloc_arrow_type(vec![ind.clone(), int]);
        let p_type = alloc_arrow_type(vec![ind, bool]);

        sig.declare_type(f, f_type.clone()).unwrap();
        assert!(!sig.is_function(f));
        sig.declare_final_type(f, f_type.clone()).unwrap();
        assert!(sig.is_function(f));
        assert!(!sig.is_predicate(f));

        sig.declare_final_type(p, p_type.clone()).unwrap();
        assert!(sig.is_predicate(p));
        assert!(!sig.is_function(p));

        let conflict = sig.declare_type(f, p_type).unwrap_err();
        assert_eq!(conflict.code(), ErrorCode::TYPE_ERROR);
    }

    #[test]
    fn special_flags_alpha_ranks_and_fof_ops_match_c_shapes() {
        let mut sig = signature();
        let zed = sig.insert_id_for_problem("zed", 0, false, ProblemType::FirstOrder);
        let alpha = sig.insert_id_for_problem("alpha", 0, false, ProblemType::FirstOrder);
        let and = sig.insert_fof_op("$and", 2);

        assert!(sig.query_prop(and, FP_FOF_OP | FP_SPECIAL));
        assert!(sig.get_alpha_rank(alpha) < sig.get_alpha_rank(zed));
        sig.set_all_special(false);
        assert!(!sig.is_special(SIG_TRUE_CODE));
        assert!(!sig.is_special(and));
        sig.set_special(alpha, true);
        assert!(sig.is_special(alpha));
    }

    #[test]
    fn arity_statistics_follow_internal_symbol_and_special_filters() {
        let mut sig = signature();
        let f = sig.insert_id_for_problem("f", 2, false, ProblemType::FirstOrder);
        let g = sig.insert_id_for_problem("g", 3, false, ProblemType::FirstOrder);
        let p = sig.insert_id_for_problem("p", 1, false, ProblemType::FirstOrder);
        let hidden = sig.insert_id_for_problem("hidden", 4, true, ProblemType::FirstOrder);

        let ind = alloc_simple_sort(ST_INDIVIDUALS);
        let bool = alloc_simple_sort(ST_BOOL);
        sig.declare_final_type(
            f,
            alloc_arrow_type(vec![ind.clone(), ind.clone(), ind.clone()]),
        )
        .unwrap();
        sig.declare_final_type(
            g,
            alloc_arrow_type(vec![ind.clone(), ind.clone(), ind.clone(), ind.clone()]),
        )
        .unwrap();
        sig.declare_final_type(p, alloc_arrow_type(vec![ind.clone(), bool.clone()]))
            .unwrap();
        sig.declare_final_type(
            hidden,
            alloc_arrow_type(vec![ind.clone(), ind.clone(), ind.clone(), ind, bool]),
        )
        .unwrap();

        assert_eq!(sig.find_max_used_arity(), 4);
        assert_eq!(sig.find_max_function_arity(), 3);
        assert_eq!(sig.find_min_function_arity(), 2);
        assert_eq!(sig.find_max_predicate_arity(), 1);
        assert_eq!(sig.find_min_predicate_arity(), 1);
        assert_eq!(sig.count_arity_symbols(2, false), 1);
        assert_eq!(sig.count_arity_symbols(1, true), 1);
        assert_eq!(sig.count_symbols(false), 2);
        assert_eq!(sig.count_symbols(true), 1);
    }

    #[test]
    fn feature_offsets_are_arity_limited_and_shifted_for_predicates() {
        let mut sig = signature();
        let f = sig.insert_id_for_problem("f", 9, false, ProblemType::FirstOrder);
        let p = sig.insert_id_for_problem("p", 7, false, ProblemType::FirstOrder);
        let ind = alloc_simple_sort(ST_INDIVIDUALS);
        let bool = alloc_simple_sort(ST_BOOL);

        sig.declare_final_type(f, alloc_arrow_type(vec![ind.clone(), ind.clone()]))
            .unwrap();
        sig.declare_final_type(p, alloc_arrow_type(vec![ind, bool]))
            .unwrap();

        assert_eq!(sig.get_feature_offset(f), 5);
        assert_eq!(sig.get_feature_offset(p), 11);
        assert_eq!(sig.get_depth_feature_offset(p), 23);
    }
}
