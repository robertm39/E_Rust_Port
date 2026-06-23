use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pdarrays::PDIntArray;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::terms::functypes::FunCode;
use crate::terms::simpletypes::{
    alloc_arrow_type, arrow_type_flattened, is_choice_type, type_app_encoded_name,
    type_get_max_arity, type_is_predicate, Type,
};
use crate::terms::typebanks::TypeBank;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};
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
    orn_codes: BTreeMap<i32, FunCode>,
    not_code: FunCode,
    qex_code: FunCode,
    qall_code: FunCode,
    and_code: FunCode,
    or_code: FunCode,
    impl_code: FunCode,
    equiv_code: FunCode,
    nand_code: FunCode,
    nor_code: FunCode,
    bimpl_code: FunCode,
    xor_code: FunCode,
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
            orn_codes: BTreeMap::new(),
            not_code: 0,
            qex_code: 0,
            qall_code: 0,
            and_code: 0,
            or_code: 0,
            impl_code: 0,
            equiv_code: 0,
            nand_code: 0,
            nor_code: 0,
            bimpl_code: 0,
            xor_code: 0,
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
    pub const fn not_code(&self) -> FunCode {
        self.not_code
    }

    #[must_use]
    pub const fn qex_code(&self) -> FunCode {
        self.qex_code
    }

    #[must_use]
    pub const fn qall_code(&self) -> FunCode {
        self.qall_code
    }

    #[must_use]
    pub const fn and_code(&self) -> FunCode {
        self.and_code
    }

    #[must_use]
    pub const fn or_code(&self) -> FunCode {
        self.or_code
    }

    #[must_use]
    pub const fn impl_code(&self) -> FunCode {
        self.impl_code
    }

    #[must_use]
    pub const fn equiv_code(&self) -> FunCode {
        self.equiv_code
    }

    #[must_use]
    pub const fn nand_code(&self) -> FunCode {
        self.nand_code
    }

    #[must_use]
    pub const fn nor_code(&self) -> FunCode {
        self.nor_code
    }

    #[must_use]
    pub const fn bimpl_code(&self) -> FunCode {
        self.bimpl_code
    }

    #[must_use]
    pub const fn xor_code(&self) -> FunCode {
        self.xor_code
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

    /// Marks a symbol as occurring in function position.
    ///
    /// # Panics
    ///
    /// Panics if the symbol has no declared type, matching the C assertion
    /// precondition. It can also panic through `TypeBank::change_return_type`
    /// for the inherited `$o` ambiguity case documented in `DOCS.md`.
    pub fn declare_is_function(&mut self, f_code: FunCode) -> Result<(), Diagnostic> {
        if self.is_polymorphic(f_code) {
            return Ok(());
        }

        let type_ = self
            .get_type(f_code)
            .cloned()
            .expect("function symbol must have a type before declaration");
        if type_.is_bool() {
            let default_sort = self.type_bank.default_type();
            let new_type = self.type_bank.change_return_type(&type_, &default_sort);
            self.declare_final_type(f_code, new_type)
        } else {
            self.fix_type(f_code);
            Ok(())
        }
    }

    /// Marks a symbol as occurring in predicate position.
    ///
    /// # Panics
    ///
    /// Panics if the symbol has no declared type, matching the C assertion
    /// precondition.
    pub fn declare_is_predicate(&mut self, f_code: FunCode) -> Result<(), Diagnostic> {
        if self.is_polymorphic(f_code) {
            return Ok(());
        }

        let type_ = self
            .get_type(f_code)
            .cloned()
            .expect("predicate symbol must have a type before declaration");
        if type_.is_bool() {
            self.fix_type(f_code);
            Ok(())
        } else {
            let bool_type = self.type_bank.bool_type();
            let new_type = self.type_bank.change_return_type(&type_, &bool_type);
            self.declare_final_type(f_code, new_type)
        }
    }

    /// Inserts the fixed block of internal FOF and higher-order helper symbols.
    ///
    /// # Panics
    ///
    /// Panics if the signature was initialized with list support. The C source
    /// has fixed-code assertions in this path that are only coherent for the
    /// default no-list-support initialization.
    pub fn insert_internal_codes(&mut self) -> Result<(), Diagnostic> {
        assert_eq!(
            self.internal_symbols, SIG_FALSE_CODE,
            "C fixed-code assertions only hold for signatures without list support"
        );

        let bool_type = self.type_bank.bool_type();
        let unary_log_op_type = self
            .type_bank
            .insert_type_shared(alloc_arrow_type(vec![bool_type.clone(), bool_type.clone()]));
        let binary_log_op_type = self.type_bank.insert_type_shared(alloc_arrow_type(vec![
            bool_type.clone(),
            bool_type.clone(),
            bool_type.clone(),
        ]));

        self.eqn_code = self.insert_id("$eq", 2, true);
        self.set_polymorphic(self.eqn_code, true);
        self.neqn_code = self.insert_id("$neq", 2, true);
        self.set_polymorphic(self.neqn_code, true);
        self.qex_code = self.insert_id("$qex", 2, true);
        self.qall_code = self.insert_id("$qall", 2, true);
        self.set_polymorphic(self.qex_code, true);
        self.set_polymorphic(self.qall_code, true);

        self.not_code = self.insert_fof_op("$not", 1);
        self.declare_final_type(self.not_code, unary_log_op_type)?;
        self.and_code = self.insert_fof_op("$and", 2);
        self.declare_final_type(self.and_code, binary_log_op_type.clone())?;
        self.or_code = self.insert_fof_op("$or", 2);
        self.declare_final_type(self.or_code, binary_log_op_type.clone())?;
        self.impl_code = self.insert_fof_op("$impl", 2);
        self.declare_final_type(self.impl_code, binary_log_op_type.clone())?;
        self.equiv_code = self.insert_fof_op("$equiv", 2);
        self.declare_final_type(self.equiv_code, binary_log_op_type.clone())?;
        self.nand_code = self.insert_fof_op("$nand", 2);
        self.declare_final_type(self.nand_code, binary_log_op_type.clone())?;
        self.nor_code = self.insert_fof_op("$nor", 2);
        self.declare_final_type(self.nor_code, binary_log_op_type.clone())?;
        self.bimpl_code = self.insert_fof_op("$bimpl", 2);
        self.declare_final_type(self.bimpl_code, binary_log_op_type.clone())?;
        self.xor_code = self.insert_fof_op("$xor", 2);
        self.declare_final_type(self.xor_code, binary_log_op_type)?;

        self.answer_code = self.insert_id("$answer", 1, true);
        self.set_func_prop(self.answer_code, FP_INTERPRETED | FP_PSEUDO_PRED);

        let phony = self.insert_id("$@_var", 1, true);
        debug_assert_eq!(phony, SIG_PHONY_APP_CODE);
        let named_lambda = self.insert_id("$named_lam", 2, true);
        debug_assert_eq!(named_lambda, SIG_NAMED_LAMBDA_CODE);
        let db_lambda = self.insert_id("$db_lam", 2, true);
        debug_assert_eq!(db_lambda, SIG_DB_LAMBDA_CODE);
        let ite = self.insert_id("$ite", 3, true);
        debug_assert_eq!(ite, SIG_ITE_CODE);
        let let_code = self.insert_id("$let", 3, true);
        debug_assert_eq!(let_code, SIG_LET_CODE);

        let answer_type = self.type_bank.insert_type_shared(alloc_arrow_type(vec![
            self.type_bank.i_type(),
            self.type_bank.bool_type(),
        ]));
        self.declare_final_type(self.answer_code, answer_type)?;

        self.distinct_code = self.insert_id("$distinct", -1, true);
        self.set_polymorphic(self.distinct_code, true);
        self.internal_symbols = self.f_count;
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

    #[must_use]
    pub const fn is_simple_answer_pred(&self, f_code: FunCode) -> bool {
        f_code == self.answer_code
    }

    /// Checks whether `f_code` is one of the built-in logical symbols.
    ///
    /// # Panics
    ///
    /// Panics if `f_code` is not a positive function code, matching the C
    /// macro assertion.
    #[must_use]
    pub fn is_logical_symbol(&self, f_code: FunCode) -> bool {
        assert!(
            f_code > 0,
            "logical-symbol checks require a positive f-code"
        );
        self.query_prop(f_code, FP_FOF_OP)
            || f_code == SIG_TRUE_CODE
            || f_code == SIG_FALSE_CODE
            || f_code == self.eqn_code
            || f_code == self.neqn_code
            || f_code == self.qex_code
            || f_code == self.qall_code
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

    pub fn get_eqn_code(&mut self, positive: bool) -> FunCode {
        if positive {
            if self.eqn_code == 0 {
                self.eqn_code = self.insert_id("$eq", 2, true);
                self.set_func_prop(self.eqn_code, FP_FOF_OP | FP_TYPE_POLY);
            }
            self.eqn_code
        } else {
            if self.neqn_code == 0 {
                self.neqn_code = self.insert_id("$neq", 2, true);
                self.set_func_prop(self.neqn_code, FP_FOF_OP | FP_TYPE_POLY);
            }
            self.neqn_code
        }
    }

    /// Returns the opposite equality/inequality code.
    ///
    /// # Panics
    ///
    /// Panics if `f_code` is neither the stored equality code nor the stored
    /// inequality code, matching the C assertion.
    #[must_use]
    pub fn get_other_eqn_code(&self, f_code: FunCode) -> FunCode {
        if f_code == self.eqn_code {
            self.neqn_code
        } else {
            assert_eq!(f_code, self.neqn_code, "expected equality code");
            self.eqn_code
        }
    }

    pub fn get_or_code(&mut self) -> FunCode {
        if self.or_code == 0 {
            self.or_code = self.insert_id("$or", 2, true);
        }
        self.or_code
    }

    pub fn get_cnil_code(&mut self) -> FunCode {
        if self.cnil_code == 0 {
            self.cnil_code = self.insert_id("$cnil", 0, true);
        }
        self.cnil_code
    }

    pub fn get_or_n_code(&mut self, arity: i32) -> FunCode {
        if let Some(code) = self.orn_codes.get(&arity) {
            return *code;
        }
        let code = self.insert_id(&format!("$or{arity}"), arity, true);
        self.orn_codes.insert(arity, code);
        code
    }

    pub fn get_new_f_code(
        &mut self,
        arity: i32,
        prefix: &str,
        counter: &mut i64,
        props: FunctionProperties,
    ) -> FunCode {
        loop {
            *counter += 1;
            let name = format!("{prefix}{counter}_{arity}");
            if self.find_f_code(&name) == 0 {
                let code = self.insert_id(&name, arity, false);
                self.set_func_prop(code, props);
                return code;
            }
        }
    }

    /// Returns the generated binary typed-application symbol for `(arg1, arg2) -> ret`.
    ///
    /// # Panics
    ///
    /// Panics if declaring the generated type fails. The generated name is
    /// derived from the three type UIDs, so a fresh declaration is expected to
    /// be compatible with any existing symbol of the same name.
    pub fn get_typed_app(&mut self, arg1: &Type, arg2: &Type, ret: &Type) -> FunCode {
        let name = format!(
            "app_{}_{}_{}",
            arg1.type_uid(),
            arg2.type_uid(),
            ret.type_uid()
        );
        let app_type = self.type_bank.insert_type_shared(alloc_arrow_type(vec![
            arg1.clone(),
            arg2.clone(),
            ret.clone(),
        ]));
        let f_code = self.insert_id(&name, 2, false);
        if self.get_type(f_code).is_none() {
            self.declare_type(f_code, app_type)
                .expect("fresh typed-application type declaration succeeds");
        }
        self.set_func_prop(f_code, FP_TYPED_APPLICATION);
        f_code
    }

    /// Prints type declarations for all external symbols after application encoding.
    ///
    /// # Panics
    ///
    /// Panics if an external symbol has no declared type, or if a typed
    /// application symbol's type does not have the expected three arrow
    /// entries. This matches the C helper's preconditions.
    pub fn print_app_encoded_decls(&self, output: &mut impl Write) -> io::Result<()> {
        for f_code in self.external_f_codes() {
            let decl_no = (f_code + 1) - self.internal_symbols;
            let name = self
                .find_name(f_code)
                .expect("external function code has a printable name");
            write!(output, "tff(symboltypedecl{decl_no}, type, {name}: ")?;
            let type_ = self
                .get_type(f_code)
                .expect("app-encoded declaration printing requires symbol types");

            if self.query_prop(f_code, FP_TYPED_APPLICATION) {
                let args = type_.args();
                let left = type_app_encoded_name(
                    args.first()
                        .expect("typed application type has left argument"),
                )
                .map_err(|diagnostic| diagnostic_to_io(&diagnostic))?;
                let right = type_app_encoded_name(
                    args.get(1)
                        .expect("typed application type has right argument"),
                )
                .map_err(|diagnostic| diagnostic_to_io(&diagnostic))?;
                let ret = type_app_encoded_name(
                    args.get(2)
                        .expect("typed application type has return argument"),
                )
                .map_err(|diagnostic| diagnostic_to_io(&diagnostic))?;
                write!(output, "({left} * {right}) > {ret}")?;
            } else {
                let type_name = type_app_encoded_name(type_)
                    .map_err(|diagnostic| diagnostic_to_io(&diagnostic))?;
                write!(output, "{type_name}")?;
            }
            output.write_all(b").\n")?;
        }
        Ok(())
    }

    pub fn get_new_skolem_code(&mut self, arity: i32) -> FunCode {
        let mut counter = self.skolem_count;
        let code = self.get_new_f_code(arity, "esk", &mut counter, FP_SKOLEM_SYMBOL);
        self.skolem_count = counter;
        code
    }

    pub fn get_new_predicate_code(&mut self, arity: i32) -> FunCode {
        let mut counter = self.newpred_count;
        let code = self.get_new_f_code(arity, "epred", &mut counter, FP_DEF_PRED);
        self.newpred_count = counter;
        code
    }

    pub fn get_new_def_code(&mut self, arity: i32) -> FunCode {
        let mut counter = self.newdef_count;
        let code = self.get_new_f_code(arity, "edef", &mut counter, FP_DEF_FUN);
        self.newdef_count = counter;
        code
    }

    /// Returns a fresh typed generated symbol and declares its type.
    ///
    /// # Panics
    ///
    /// Panics if the flattened type arity does not fit in a C `int`/Rust
    /// `i32`, matching the inherited signature arity representation.
    pub fn get_new_typed_f_code(
        &mut self,
        prefix: &str,
        args: &[Type],
        counter: &mut i64,
        ret_type: &Type,
        props: FunctionProperties,
    ) -> Result<FunCode, Diagnostic> {
        let symbol_type = self
            .type_bank
            .insert_type_shared(arrow_type_flattened(args, ret_type));
        let max_arity = i32::try_from(type_get_max_arity(&symbol_type))
            .expect("typed symbol arity fits in i32");
        let f_code = self.get_new_f_code(max_arity, prefix, counter, props);
        self.declare_type(f_code, symbol_type.clone())?;
        if type_is_predicate(&symbol_type) {
            self.declare_is_predicate(f_code)?;
        } else {
            self.declare_is_function(f_code)?;
        }
        Ok(f_code)
    }

    pub fn get_new_typed_skolem(
        &mut self,
        args: &[Type],
        ret_type: &Type,
    ) -> Result<FunCode, Diagnostic> {
        let mut counter = self.newpred_count;
        let code = self.get_new_typed_f_code("esk", args, &mut counter, ret_type, FP_DEF_PRED)?;
        self.newpred_count = counter;
        Ok(code)
    }

    pub fn get_new_typed_def_code(
        &mut self,
        args: &[Type],
        ret_type: &Type,
    ) -> Result<FunCode, Diagnostic> {
        let mut counter = self.newdef_count;
        let code = self.get_new_typed_f_code("edef", args, &mut counter, ret_type, FP_DEF_FUN)?;
        self.newdef_count = counter;
        Ok(code)
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

    /// Adds selected symbol arities into `distrib` and returns the maximum.
    ///
    /// This mirrors C `SigAddSymbolArities`: unlike most signature statistics,
    /// it scans all f-codes, including internal symbols, and relies on the
    /// caller-provided `selection` array to choose the relevant symbols.
    ///
    /// # Panics
    ///
    /// Panics if a selected symbol has a negative arity or an arity that cannot
    /// be represented as a dynamic-array index.
    pub fn add_symbol_arities(
        &self,
        distrib: &mut PDIntArray,
        predicates: bool,
        selection: &[i64],
    ) -> i32 {
        let mut max_arity = -1;
        for f_code in 1..=self.f_count {
            let selection_index =
                usize::try_from(f_code).expect("positive f-code fits selection index");
            if self.is_predicate(f_code) == predicates
                && selection.get(selection_index).copied().unwrap_or(0) != 0
            {
                let arity = self.find_arity(f_code).expect("valid f-code has arity");
                max_arity = max_arity.max(arity);
                let arity_index =
                    isize::try_from(arity).expect("selected arity fits dynamic-array index");
                assert!(
                    distrib.inc_int(arity_index, 1).is_some(),
                    "arity distribution accepts selected arity"
                );
            }
        }
        max_arity
    }

    #[must_use]
    pub fn has_unimplemented_interpreted_symbols(&self) -> bool {
        self.external_f_codes()
            .any(|f_code| self.query_prop(f_code, FP_INTERPRETED))
    }

    /// Checks for a declared choice-symbol type among external symbols.
    ///
    /// # Panics
    ///
    /// Panics if an external symbol has no type, matching the C implementation's
    /// direct `SigGetType` dereference in this path.
    #[must_use]
    pub fn has_choice_sym(&self) -> bool {
        self.external_f_codes().any(|f_code| {
            is_choice_type(
                self.get_type(f_code)
                    .expect("choice-symbol scan requires external symbol types"),
            )
        })
    }

    /// Mirrors C `SigSymbolUnifiesWithVar`.
    ///
    /// The C condition contains broad disjuncts, including
    /// `f_code != SIG_DB_LAMBDA_CODE`, so most nonzero symbols return `true`.
    ///
    /// # Panics
    ///
    /// Panics if `f_code` is zero, matching the C assertion.
    #[must_use]
    pub fn symbol_unifies_with_var(&self, f_code: FunCode) -> bool {
        assert_ne!(
            f_code, 0,
            "variable-unification symbol code must be nonzero"
        );
        problem_type() == ProblemType::HigherOrder
            || f_code == SIG_TRUE_CODE
            || f_code == SIG_FALSE_CODE
            || f_code != SIG_DB_LAMBDA_CODE
            || f_code <= 0
            || !self.is_predicate(f_code)
    }

    pub fn fcodes_collect_types(&self, fcodes: &BTreeSet<FunCode>, types: &mut Vec<Type>) -> i64 {
        let mut count = 0;
        let mut to_process = Vec::new();
        for f_code in fcodes {
            if let Some(type_) = self.get_type(*f_code) {
                to_process.push(type_.clone());
            }
        }

        while let Some(type_) = to_process.pop() {
            if !types.iter().any(|existing| existing == &type_) {
                count += 1;
                types.push(type_.clone());
                to_process.extend(type_.args().iter().cloned());
            }
        }
        count
    }

    #[must_use]
    pub fn collect_sort_consts(&self, sort: &Type) -> Vec<FunCode> {
        let default_sort = self.type_bank.i_type();
        self.external_f_codes()
            .filter(|&f_code| self.find_arity(f_code) == Some(0))
            .filter(|&f_code| {
                let symbol_sort = self
                    .get_type(f_code)
                    .cloned()
                    .unwrap_or(default_sort.clone());
                symbol_sort == *sort
            })
            .collect()
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

fn diagnostic_to_io(diagnostic: &Diagnostic) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, diagnostic.message().to_owned())
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
    use crate::basics::pdarrays::{PDIntArray, GROW_EXPONENTIAL};
    use crate::basics::simple_stuff::ProblemType;
    use crate::terms::simpletypes::{
        alloc_arrow_type, alloc_simple_sort, Type, ST_BOOL, ST_INDIVIDUALS, ST_INTEGER,
    };
    use crate::terms::typebanks::TypeBank;
    use std::collections::BTreeSet;

    fn signature() -> Signature {
        Signature::new(TypeBank::new())
    }

    fn string_from(bytes: Vec<u8>) -> String {
        String::from_utf8(bytes).unwrap()
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
    fn add_symbol_arities_scans_selected_internal_and_external_symbols() {
        let mut sig = signature();
        let f = sig.insert_id_for_problem("f", 2, false, ProblemType::FirstOrder);
        let p = sig.insert_id_for_problem("p", 1, false, ProblemType::FirstOrder);
        let individual = sig.type_bank().i_type();
        let bool_type = sig.type_bank().bool_type();
        sig.declare_final_type(
            f,
            alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
                individual.clone(),
            ]),
        )
        .unwrap();
        sig.declare_final_type(p, alloc_arrow_type(vec![individual, bool_type]))
            .unwrap();
        let mut selection = vec![0; usize::try_from(sig.f_count() + 1).unwrap()];
        selection[usize::try_from(SIG_TRUE_CODE).unwrap()] = 1;
        selection[usize::try_from(f).unwrap()] = 1;
        selection[usize::try_from(p).unwrap()] = 1;

        let mut predicate_dist = PDIntArray::new_int(1, GROW_EXPONENTIAL);
        assert_eq!(
            sig.add_symbol_arities(&mut predicate_dist, true, &selection),
            1
        );
        assert_eq!(predicate_dist.element_int(0), 1);
        assert_eq!(predicate_dist.element_int(1), 1);

        let mut function_dist = PDIntArray::new_int(1, GROW_EXPONENTIAL);
        assert_eq!(
            sig.add_symbol_arities(&mut function_dist, false, &selection),
            2
        );
        assert_eq!(function_dist.element_int(2), 1);
        assert_eq!(function_dist.element_int(0), 0);
    }

    #[test]
    fn fcodes_collect_types_collects_declared_types_and_subtypes_once() {
        let mut sig = signature();
        let individual = sig.type_bank().i_type();
        let bool_type = sig.type_bank().bool_type();
        let animal_code = sig.type_bank_mut().define_simple_sort("$animal").unwrap();
        let animal = sig
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(animal_code));
        let f_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                animal.clone(),
                bool_type.clone(),
            ]));
        let p_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![animal.clone(), bool_type.clone()]));
        let f = sig.insert_id_for_problem("f", 2, false, ProblemType::FirstOrder);
        sig.declare_final_type(f, f_type.clone()).unwrap();
        let p = sig.insert_id_for_problem("p", 1, false, ProblemType::FirstOrder);
        sig.declare_final_type(p, p_type.clone()).unwrap();
        let untyped = sig.insert_id_for_problem("untyped", 0, false, ProblemType::FirstOrder);

        let mut fcodes = BTreeSet::new();
        fcodes.insert(f);
        fcodes.insert(p);
        fcodes.insert(untyped);
        let mut types = Vec::new();

        assert_eq!(sig.fcodes_collect_types(&fcodes, &mut types), 5);
        assert_eq!(
            types,
            vec![
                p_type,
                bool_type.clone(),
                animal.clone(),
                f_type,
                individual
            ]
        );
        assert_eq!(sig.fcodes_collect_types(&fcodes, &mut types), 0);
    }

    #[test]
    fn collect_sort_consts_uses_insertion_order_and_default_individual_sort() {
        let mut sig = signature();
        let individual = sig.type_bank().i_type();
        let animal_code = sig.type_bank_mut().define_simple_sort("$animal").unwrap();
        let animal = sig
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(animal_code));

        let untyped = sig.insert_id_for_problem("untyped", 0, false, ProblemType::FirstOrder);
        let typed_individual =
            sig.insert_id_for_problem("typed_individual", 0, false, ProblemType::FirstOrder);
        sig.declare_final_type(typed_individual, individual.clone())
            .unwrap();
        let typed_animal =
            sig.insert_id_for_problem("typed_animal", 0, false, ProblemType::FirstOrder);
        sig.declare_final_type(typed_animal, animal.clone())
            .unwrap();
        let unary = sig.insert_id_for_problem("unary", 1, false, ProblemType::FirstOrder);
        sig.declare_final_type(
            unary,
            alloc_arrow_type(vec![individual.clone(), individual.clone()]),
        )
        .unwrap();

        assert_eq!(
            sig.collect_sort_consts(&individual),
            vec![untyped, typed_individual]
        );
        assert_eq!(sig.collect_sort_consts(&animal), vec![typed_animal]);
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

    #[test]
    fn insert_internal_codes_adds_fof_symbols_and_fixed_code_block() {
        let mut sig = signature();

        sig.insert_internal_codes().unwrap();

        assert_eq!(sig.eqn_code(), 3);
        assert_eq!(sig.neqn_code(), 4);
        assert_eq!(sig.qex_code(), 5);
        assert_eq!(sig.qall_code(), 6);
        assert_eq!(sig.not_code(), 7);
        assert_eq!(sig.and_code(), 8);
        assert_eq!(sig.or_code(), 9);
        assert_eq!(sig.impl_code(), 10);
        assert_eq!(sig.equiv_code(), 11);
        assert_eq!(sig.nand_code(), 12);
        assert_eq!(sig.nor_code(), 13);
        assert_eq!(sig.bimpl_code(), 14);
        assert_eq!(sig.xor_code(), 15);
        assert_eq!(sig.answer_code(), 16);
        assert_eq!(sig.find_f_code("$@_var"), SIG_PHONY_APP_CODE);
        assert_eq!(sig.find_f_code("$named_lam"), SIG_NAMED_LAMBDA_CODE);
        assert_eq!(sig.find_f_code("$db_lam"), SIG_DB_LAMBDA_CODE);
        assert_eq!(sig.find_f_code("$ite"), SIG_ITE_CODE);
        assert_eq!(sig.find_f_code("$let"), SIG_LET_CODE);
        assert_eq!(sig.distinct_code(), 22);
        assert_eq!(sig.internal_symbols(), sig.f_count());

        assert!(sig.is_polymorphic(sig.eqn_code()));
        assert!(sig.is_polymorphic(sig.neqn_code()));
        assert!(sig.is_polymorphic(sig.qex_code()));
        assert!(sig.is_polymorphic(sig.qall_code()));
        assert!(sig.is_polymorphic(sig.distinct_code()));
        assert!(sig.query_prop(sig.not_code(), FP_FOF_OP | FP_SPECIAL | FP_TYPE_FIXED));
        assert!(sig.query_prop(
            sig.answer_code(),
            FP_INTERPRETED | FP_PSEUDO_PRED | FP_TYPE_FIXED | FP_SPECIAL
        ));
        assert!(sig.get_type(sig.answer_code()).is_some());
    }

    #[test]
    fn lazy_special_code_helpers_create_and_reuse_symbols() {
        let mut sig = signature();

        let eq = sig.get_eqn_code(true);
        let neq = sig.get_eqn_code(false);
        assert_eq!(sig.get_eqn_code(true), eq);
        assert_eq!(sig.get_eqn_code(false), neq);
        assert_eq!(sig.get_other_eqn_code(eq), neq);
        assert_eq!(sig.get_other_eqn_code(neq), eq);
        assert!(sig.query_prop(eq, FP_FOF_OP | FP_TYPE_POLY));
        assert!(sig.query_prop(neq, FP_FOF_OP | FP_TYPE_POLY));

        let or = sig.get_or_code();
        assert_eq!(sig.get_or_code(), or);
        assert_eq!(sig.find_f_code("$or"), or);

        let cnil = sig.get_cnil_code();
        assert_eq!(sig.get_cnil_code(), cnil);
        assert_eq!(sig.find_f_code("$cnil"), cnil);

        let or3 = sig.get_or_n_code(3);
        assert_eq!(sig.get_or_n_code(3), or3);
        assert_eq!(sig.find_f_code("$or3"), or3);
        assert_eq!(sig.find_arity(or3), Some(3));
    }

    #[test]
    fn generated_symbol_helpers_increment_counters_and_skip_existing_names() {
        let mut sig = signature();
        sig.insert_id_for_problem("esk1_2", 2, false, ProblemType::FirstOrder);

        let skolem = sig.get_new_skolem_code(2);
        assert_eq!(sig.find_name(skolem), Some("esk2_2"));
        assert_eq!(sig.skolem_count(), 2);
        assert!(sig.query_prop(skolem, FP_SKOLEM_SYMBOL));

        let predicate = sig.get_new_predicate_code(1);
        assert_eq!(sig.find_name(predicate), Some("epred1_1"));
        assert_eq!(sig.newpred_count(), 1);
        assert!(sig.query_prop(predicate, FP_DEF_PRED));

        let def = sig.get_new_def_code(0);
        assert_eq!(sig.find_name(def), Some("edef1_0"));
        assert_eq!(sig.newdef_count(), 1);
        assert!(sig.query_prop(def, FP_DEF_FUN));
    }

    #[test]
    fn typed_generated_symbol_helpers_declare_types_and_preserve_c_counters() {
        let mut sig = signature();
        let individual = sig.type_bank().i_type();
        let bool_type = sig.type_bank().bool_type();

        let typed_skolem = sig
            .get_new_typed_skolem(std::slice::from_ref(&individual), &individual)
            .unwrap();
        assert_eq!(sig.find_name(typed_skolem), Some("esk1_1"));
        assert_eq!(sig.newpred_count(), 1);
        assert_eq!(sig.skolem_count(), 0);
        assert!(sig.query_prop(typed_skolem, FP_DEF_PRED | FP_TYPE_FIXED));
        assert!(sig.is_function(typed_skolem));
        assert_eq!(
            sig.get_type(typed_skolem)
                .expect("typed generated symbol has type")
                .args(),
            &[individual.clone(), individual.clone()]
        );

        let typed_def = sig
            .get_new_typed_def_code(std::slice::from_ref(&individual), &bool_type)
            .unwrap();
        assert_eq!(sig.find_name(typed_def), Some("edef1_1"));
        assert_eq!(sig.newdef_count(), 1);
        assert!(sig.query_prop(typed_def, FP_DEF_FUN | FP_TYPE_FIXED));
        assert!(sig.is_predicate(typed_def));
        assert_eq!(
            sig.get_type(typed_def)
                .expect("typed generated predicate has type")
                .args(),
            &[individual, bool_type]
        );
    }

    #[test]
    fn typed_application_symbols_are_named_by_type_uids_and_reused() {
        let mut sig = signature();
        let individual = sig.type_bank().i_type();
        let bool_type = sig.type_bank().bool_type();
        let unary_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                bool_type.clone(),
            ]));
        let expected_name = format!(
            "app_{}_{}_{}",
            unary_type.type_uid(),
            individual.type_uid(),
            bool_type.type_uid()
        );

        let app = sig.get_typed_app(&unary_type, &individual, &bool_type);

        assert_eq!(sig.find_name(app), Some(expected_name.as_str()));
        assert_eq!(sig.find_arity(app), Some(2));
        assert!(sig.query_prop(app, FP_TYPED_APPLICATION));
        assert_eq!(
            sig.get_type(app)
                .expect("typed app has declared type")
                .args(),
            &[unary_type.clone(), individual.clone(), bool_type.clone()]
        );
        assert_eq!(sig.get_typed_app(&unary_type, &individual, &bool_type), app);
    }

    #[test]
    fn app_encoded_decls_print_external_symbol_types() {
        let mut sig = signature();
        let individual = sig.type_bank().i_type();
        let bool_type = sig.type_bank().bool_type();
        let predicate_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                bool_type.clone(),
            ]));
        let predicate = sig.insert_id_for_problem("p", 1, false, ProblemType::FirstOrder);
        sig.declare_final_type(predicate, predicate_type.clone())
            .unwrap();
        let app = sig.get_typed_app(&predicate_type, &individual, &bool_type);
        let app_name = sig.find_name(app).unwrap();

        let mut output = Vec::new();
        sig.print_app_encoded_decls(&mut output).unwrap();

        assert_eq!(
            string_from(output),
            format!(
                "tff(symboltypedecl2, type, p: type_{}).\n\
                 tff(symboltypedecl3, type, {app_name}: (type_{} * $i) > $o).\n",
                predicate_type.type_uid(),
                predicate_type.type_uid()
            )
        );
    }

    #[test]
    fn signature_predicate_helpers_follow_c_macro_shapes() {
        let mut sig = signature();
        sig.insert_internal_codes().unwrap();
        let individual = sig.type_bank().i_type();
        let bool_type = sig.type_bank().bool_type();
        let pred = sig.insert_id_for_problem("p", 1, false, ProblemType::FirstOrder);
        sig.declare_final_type(pred, alloc_arrow_type(vec![individual, bool_type]))
            .unwrap();

        assert!(sig.is_simple_answer_pred(sig.answer_code()));
        assert!(!sig.is_simple_answer_pred(pred));
        assert!(sig.is_logical_symbol(SIG_TRUE_CODE));
        assert!(sig.is_logical_symbol(SIG_FALSE_CODE));
        assert!(sig.is_logical_symbol(sig.eqn_code()));
        assert!(sig.is_logical_symbol(sig.qex_code()));
        assert!(sig.is_logical_symbol(sig.not_code()));
        assert!(!sig.is_logical_symbol(sig.answer_code()));

        assert!(!sig.has_unimplemented_interpreted_symbols());
        let interpreted = sig.insert_id_for_problem("$custom", 0, false, ProblemType::FirstOrder);
        sig.set_func_prop(interpreted, FP_INTERPRETED);
        assert!(sig.has_unimplemented_interpreted_symbols());

        assert!(sig.symbol_unifies_with_var(pred));
        assert!(sig.symbol_unifies_with_var(-2));
    }

    #[test]
    fn choice_symbol_scan_uses_declared_external_types() {
        let mut sig = signature();
        assert!(!sig.has_choice_sym());

        let individual = sig.type_bank().i_type();
        let bool_type = sig.type_bank().bool_type();
        let predicate = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                bool_type.clone(),
            ]));
        let choice_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![predicate, individual.clone()]));
        let choice = sig.insert_id_for_problem("choice", 1, false, ProblemType::FirstOrder);
        sig.declare_final_type(choice, choice_type).unwrap();

        assert!(sig.has_choice_sym());
    }
}
