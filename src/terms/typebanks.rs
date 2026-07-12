use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::inout::scanner::{Scanner, TokenType};
use crate::terms::functypes::{func_symb_parse, FuncSymbType};
use crate::terms::simpletypes::{
    alloc_arrow_type, alloc_simple_sort, sort_is_user_defined, type_alloc, type_app_encoded_name,
    Type, TypeConsCode, TypeUniqueId, ARROW_TYPE_CONS, INVALID_TYPE_UID, ST_BOOL, ST_INDIVIDUALS,
    ST_INTEGER, ST_KIND, ST_RATIONAL, ST_REAL,
};
use std::collections::BTreeMap;
use std::io::{self, Write};

pub const TYPEBANK_SIZE: usize = 4096;
pub const TYPEBANK_HASH_MASK: usize = TYPEBANK_SIZE - 1;
pub const NAME_NOT_FOUND: TypeConsCode = -1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeConstructorInfo {
    name: String,
    arity: usize,
}

impl TypeConstructorInfo {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn arity(&self) -> usize {
        self.arity
    }
}

#[derive(Clone, Debug)]
pub struct TypeBank {
    back_index: Vec<TypeConstructorInfo>,
    name_index: BTreeMap<String, TypeConstructorNameInfo>,
    shared_types: BTreeMap<TypeKey, Type>,
    types_count: TypeUniqueId,
    max_predefined_count: TypeUniqueId,
    bool_type: Type,
    i_type: Type,
    kind_type: Type,
    integer_type: Type,
    rational_type: Type,
    real_type: Type,
    default_type: Type,
}

impl Default for TypeBank {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeBank {
    #[must_use]
    pub fn new() -> Self {
        let constructor_defs = [
            (ARROW_TYPE_CONS, "$>_type"),
            (ST_BOOL, "$o"),
            (ST_INDIVIDUALS, "$i"),
            (ST_KIND, "$tType"),
            (ST_INTEGER, "$int"),
            (ST_RATIONAL, "$rat"),
            (ST_REAL, "$real"),
        ];
        let mut back_index = Vec::with_capacity(constructor_defs.len());
        let mut name_index = BTreeMap::new();
        for (code, name) in constructor_defs {
            debug_assert_eq!(usize::try_from(code).ok(), Some(back_index.len()));
            back_index.push(TypeConstructorInfo {
                name: name.to_owned(),
                arity: 0,
            });
            name_index.insert(name.to_owned(), TypeConstructorNameInfo { code, arity: 0 });
        }

        let placeholder = alloc_simple_sort(ST_BOOL);
        let mut bank = Self {
            back_index,
            name_index,
            shared_types: BTreeMap::new(),
            types_count: 0,
            max_predefined_count: 0,
            bool_type: placeholder.clone(),
            i_type: placeholder.clone(),
            kind_type: placeholder.clone(),
            integer_type: placeholder.clone(),
            rational_type: placeholder.clone(),
            real_type: placeholder.clone(),
            default_type: placeholder,
        };

        bank.bool_type = bank.insert_type_shared(alloc_simple_sort(ST_BOOL));
        bank.i_type = bank.insert_type_shared(alloc_simple_sort(ST_INDIVIDUALS));
        bank.kind_type = bank.insert_type_shared(alloc_simple_sort(ST_KIND));
        bank.integer_type = bank.insert_type_shared(alloc_simple_sort(ST_INTEGER));
        bank.rational_type = bank.insert_type_shared(alloc_simple_sort(ST_RATIONAL));
        bank.real_type = bank.insert_type_shared(alloc_simple_sort(ST_REAL));
        bank.default_type = bank.i_type.clone();
        bank.max_predefined_count = bank.types_count;
        bank
    }

    #[must_use]
    pub fn names_count(&self) -> usize {
        self.back_index.len()
    }

    #[must_use]
    pub const fn types_count(&self) -> TypeUniqueId {
        self.types_count
    }

    #[must_use]
    pub const fn max_predefined_count(&self) -> TypeUniqueId {
        self.max_predefined_count
    }

    #[must_use]
    pub fn bool_type(&self) -> Type {
        self.bool_type.clone()
    }

    #[must_use]
    pub fn i_type(&self) -> Type {
        self.i_type.clone()
    }

    #[must_use]
    pub fn kind_type(&self) -> Type {
        self.kind_type.clone()
    }

    #[must_use]
    pub fn integer_type(&self) -> Type {
        self.integer_type.clone()
    }

    #[must_use]
    pub fn rational_type(&self) -> Type {
        self.rational_type.clone()
    }

    #[must_use]
    pub fn real_type(&self) -> Type {
        self.real_type.clone()
    }

    #[must_use]
    pub fn default_type(&self) -> Type {
        self.default_type.clone()
    }

    #[must_use]
    pub fn find_tc_code(&self, name: &str) -> TypeConsCode {
        self.name_index
            .get(name)
            .map_or(NAME_NOT_FOUND, |info| info.code)
    }

    #[must_use]
    pub fn find_tc_arity(&self, tc_code: TypeConsCode) -> Option<usize> {
        usize::try_from(tc_code)
            .ok()
            .and_then(|index| self.back_index.get(index))
            .map(TypeConstructorInfo::arity)
    }

    #[must_use]
    pub fn find_tc_name(&self, tc_code: TypeConsCode) -> Option<&str> {
        usize::try_from(tc_code)
            .ok()
            .and_then(|index| self.back_index.get(index))
            .map(TypeConstructorInfo::name)
    }

    #[must_use]
    pub fn constructor_info(&self, tc_code: TypeConsCode) -> Option<&TypeConstructorInfo> {
        usize::try_from(tc_code)
            .ok()
            .and_then(|index| self.back_index.get(index))
    }

    pub fn define_simple_sort(&mut self, name: &str) -> Result<TypeConsCode, Diagnostic> {
        self.define_type_constructor(name, 0)
    }

    pub fn define_type_constructor(
        &mut self,
        name: &str,
        arity: usize,
    ) -> Result<TypeConsCode, Diagnostic> {
        if i32::try_from(arity).is_err() {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                "Type constructor arity is too large",
            ));
        }

        if let Some(info) = self.name_index.get(name) {
            if info.arity == arity {
                return Ok(info.code);
            }
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                format!("Redefinition of type constructor {name}"),
            ));
        }

        let code = i64::try_from(self.back_index.len()).map_err(|_| {
            Diagnostic::new(
                ErrorCode::RESOURCE_OUT,
                "Too many type constructors in type bank",
            )
        })?;
        self.back_index.push(TypeConstructorInfo {
            name: name.to_owned(),
            arity,
        });
        self.name_index
            .insert(name.to_owned(), TypeConstructorNameInfo { code, arity });
        Ok(code)
    }

    pub fn insert_type_shared(&mut self, type_: Type) -> Type {
        let shared_arg_type = self.force_arg_sharing(type_);
        let key = TypeKey::from_shared_type(&shared_arg_type);

        if shared_arg_type.type_uid() != INVALID_TYPE_UID {
            debug_assert!(
                self.shared_types
                    .get(&key)
                    .is_some_and(|stored| stored == &shared_arg_type),
                "type with initialized UID is not shared by this bank"
            );
            return shared_arg_type;
        }

        if let Some(shared) = self.shared_types.get(&key) {
            return shared.clone();
        }

        self.types_count += 1;
        shared_arg_type.set_type_uid(self.types_count);
        self.shared_types.insert(key, shared_arg_type.clone());
        shared_arg_type
    }

    pub fn parse_type_from_current_problem(
        &mut self,
        scanner: &mut Scanner,
    ) -> Result<Type, Diagnostic> {
        self.parse_type(scanner, problem_type())
    }

    pub fn parse_type(
        &mut self,
        scanner: &mut Scanner,
        problem_type: ProblemType,
    ) -> Result<Type, Diagnostic> {
        let type_ = match problem_type {
            ProblemType::FirstOrder => self.parse_fo_type(scanner)?,
            ProblemType::HigherOrder => self.parse_ho_type(scanner)?,
            ProblemType::NotInitialized => {
                return Err(Diagnostic::new(
                    ErrorCode::SYNTAX_ERROR,
                    "Problem type is not initialized",
                ));
            }
        };
        Ok(self.insert_type_shared(type_))
    }

    #[must_use]
    pub fn type_is_user_defined(&self, type_: &Type) -> bool {
        type_.type_uid() > self.max_predefined_count
    }

    pub fn print_tstp(
        &self,
        output: &mut impl Write,
        type_: &Type,
        problem_type: ProblemType,
    ) -> io::Result<()> {
        if type_.is_arrow() {
            if problem_type == ProblemType::FirstOrder {
                self.print_fo_arrow(output, type_, problem_type)?;
            } else {
                self.print_ho_arrow(output, type_, problem_type)?;
            }
            return Ok(());
        }

        output.write_all(self.tc_name_or_io(type_.f_code())?.as_bytes())?;
        if type_.arity() != 0 {
            output.write_all(b"(")?;
            for arg in &type_.args()[..type_.arity() - 1] {
                self.print_tstp(output, arg, problem_type)?;
                output.write_all(b", ")?;
            }
            self.print_tstp(output, &type_.args()[type_.arity() - 1], problem_type)?;
            output.write_all(b")")?;
        }
        Ok(())
    }

    pub fn print_selected_sort_defs<'a, I>(
        &self,
        output: &mut impl Write,
        selector: I,
        problem_type: ProblemType,
    ) -> io::Result<usize>
    where
        I: IntoIterator<Item = &'a Type>,
    {
        let tag = if problem_type == ProblemType::HigherOrder {
            "thf"
        } else {
            "tff"
        };
        let mut selected_sorts: Vec<_> = selector
            .into_iter()
            .filter(|type_| type_.arity() == 0 && self.type_is_user_defined(type_))
            .collect();
        selected_sorts.sort_by_key(|type_| type_.type_uid());
        let mut count = 0;
        for type_ in selected_sorts {
            count += 1;
            write!(output, "{tag}(decl_sort{count}, type, ")?;
            self.print_tstp(output, type_, problem_type)?;
            output.write_all(b": $tType).\n")?;
        }
        Ok(count)
    }

    /// Changes the return type of an arrow type, or maps the individual sort to bool.
    ///
    /// # Panics
    ///
    /// Panics when `type_` is neither an arrow type nor the individual sort,
    /// matching the C helper's assertion precondition.
    pub fn change_return_type(&mut self, type_: &Type, new_ret: &Type) -> Type {
        assert!(
            type_.is_arrow() || type_.f_code() == ST_INDIVIDUALS,
            "expected arrow or individual type"
        );
        if !type_.is_arrow() {
            return self.bool_type.clone();
        }

        let mut args = type_.args().to_vec();
        if let Some(ret) = args.last_mut() {
            *ret = new_ret.clone();
        }
        self.insert_type_shared(type_alloc(type_.f_code(), args))
    }

    pub fn app_encode_types(
        &self,
        output: &mut impl Write,
        problem_type: ProblemType,
        print_type_comment: bool,
    ) -> io::Result<usize> {
        let mut types: Vec<_> = self.shared_types.values().collect();
        types.sort_by_key(|type_| type_.type_uid());

        let mut total_types = 0;
        for type_ in types {
            if type_.is_arrow() || sort_is_user_defined(type_.f_code()) {
                total_types += 1;
                let type_name = type_app_encoded_name(type_)
                    .map_err(|diagnostic| diagnostic_to_io(&diagnostic))?;
                if print_type_comment {
                    output.write_all(b"%-- ")?;
                    self.print_tstp(output, type_, problem_type)?;
                    output.write_all(b".\n")?;
                }
                writeln!(
                    output,
                    "tff(typedecl{total_types}, type, {type_name}: $tType)."
                )?;
            }
        }
        Ok(total_types)
    }

    fn force_arg_sharing(&mut self, type_: Type) -> Type {
        if type_.arity() == 0 {
            return type_;
        }

        let mut changed = false;
        let mut shared_args = Vec::with_capacity(type_.arity());
        for arg in type_.args() {
            let shared = if arg.type_uid() == INVALID_TYPE_UID {
                self.insert_type_shared(arg.clone())
            } else {
                arg.clone()
            };
            changed |= &shared != arg;
            shared_args.push(shared);
        }

        if changed {
            type_alloc(type_.f_code(), shared_args)
        } else {
            type_
        }
    }

    fn parse_single_type(&mut self, scanner: &mut Scanner) -> Result<Type, Diagnostic> {
        let mut id = DynamicString::new();
        let id_type = func_symb_parse(scanner, &mut id)?;
        if !matches!(
            id_type,
            FuncSymbType::IdentFreeFun | FuncSymbType::IdentInterpreted
        ) {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                "Function identifier expected",
            ));
        }

        let name = id.view().into_owned();
        if scanner.test_tok(TokenType::OPEN_BRACKET) {
            scanner.accept_tok(TokenType::OPEN_BRACKET)?;
            let mut args = Vec::new();
            let first_arg = self.parse_single_type(scanner)?;
            ensure_not_kind(&first_arg)?;
            args.push(first_arg);

            while scanner.test_tok(TokenType::COMMA) {
                scanner.accept_tok(TokenType::COMMA)?;
                let arg = self.parse_single_type(scanner)?;
                ensure_not_kind(&arg)?;
                args.push(arg);
            }
            scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

            let tc_code = self.find_or_define_constructor(&name, args.len())?;
            Ok(self.insert_type_shared(type_alloc(tc_code, args)))
        } else {
            let tc_code = self.find_or_define_simple_sort(&name)?;
            Ok(self.insert_type_shared(alloc_simple_sort(tc_code)))
        }
    }

    fn parse_fo_type(&mut self, scanner: &mut Scanner) -> Result<Type, Diagnostic> {
        let mut open_parens = 0;
        while open_parens < 2 && scanner.test_tok(TokenType::OPEN_BRACKET) {
            scanner.accept_tok(TokenType::OPEN_BRACKET)?;
            open_parens += 1;
        }

        let mut args = vec![self.parse_single_type(scanner)?];
        while scanner.test_tok(TokenType::MULT) {
            scanner.accept_tok(TokenType::MULT)?;
            args.push(self.parse_single_type(scanner)?);
        }

        if open_parens != 0 && scanner.test_tok(TokenType::CLOSE_BRACKET) {
            scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
            open_parens -= 1;
        }

        if scanner.test_tok(TokenType::GREATER_SIGN) {
            scanner.accept_tok(TokenType::GREATER_SIGN)?;
            args.push(self.parse_single_type(scanner)?);
        }

        if open_parens != 0 {
            scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
        }

        if args.len() == 1 {
            Ok(args.remove(0))
        } else {
            Ok(alloc_arrow_type(args))
        }
    }

    fn parse_ho_type(&mut self, scanner: &mut Scanner) -> Result<Type, Diagnostic> {
        let left_arg = if scanner.test_tok(TokenType::OPEN_BRACKET) {
            scanner.accept_tok(TokenType::OPEN_BRACKET)?;
            let type_ = self.parse_type(scanner, ProblemType::HigherOrder)?;
            scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
            type_
        } else {
            self.parse_single_type(scanner)?
        };

        if scanner.test_tok(
            TokenType::CLOSE_BRACKET
                | TokenType::FULLSTOP
                | TokenType::CLOSE_SQUARE
                | TokenType::COMMA
                | TokenType::EQUAL_SIGN
                | TokenType::NEG_EQUAL_SIGN
                | TokenType::FOF_BIN_OP,
        ) {
            return Ok(left_arg);
        }

        let mut args = vec![left_arg];
        let mut right_arg;
        loop {
            if scanner.test_tok(TokenType::MULT) {
                return Err(Diagnostic::new(
                    ErrorCode::SYNTAX_ERROR,
                    "Mixing of first order and higher order syntax is forbidden",
                ));
            }
            scanner.accept_tok(TokenType::GREATER_SIGN)?;
            right_arg = if scanner.test_tok(TokenType::OPEN_BRACKET) {
                scanner.accept_tok(TokenType::OPEN_BRACKET)?;
                let type_ = self.parse_type(scanner, ProblemType::HigherOrder)?;
                scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
                type_
            } else {
                self.parse_type(scanner, ProblemType::HigherOrder)?
            };
            args.push(right_arg.clone());
            if scanner.test_tok(
                TokenType::CLOSE_BRACKET
                    | TokenType::FULLSTOP
                    | TokenType::CLOSE_SQUARE
                    | TokenType::COMMA,
            ) {
                break;
            }
        }

        if right_arg.is_arrow() {
            let last_index = args.len() - 1;
            args[last_index] = right_arg.args()[0].clone();
            args.extend(right_arg.args()[1..].iter().cloned());
        }

        Ok(alloc_arrow_type(args))
    }

    fn find_or_define_constructor(
        &mut self,
        name: &str,
        arity: usize,
    ) -> Result<TypeConsCode, Diagnostic> {
        let tc_code = self.find_tc_code(name);
        if tc_code == NAME_NOT_FOUND {
            self.define_type_constructor(name, arity)
        } else if self.find_tc_arity(tc_code) == Some(arity) {
            Ok(tc_code)
        } else {
            Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                format!(
                    "Redefinition of type constructor {}. Mismatch in number of arguments.",
                    self.find_tc_name(tc_code).unwrap_or(name)
                ),
            ))
        }
    }

    fn find_or_define_simple_sort(&mut self, name: &str) -> Result<TypeConsCode, Diagnostic> {
        let tc_code = self.find_tc_code(name);
        if tc_code == NAME_NOT_FOUND {
            self.define_simple_sort(name)
        } else if self.find_tc_arity(tc_code) == Some(0) {
            Ok(tc_code)
        } else {
            Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                format!(
                    "Type constructor {}has not been declared as simple sort.",
                    self.find_tc_name(tc_code).unwrap_or(name)
                ),
            ))
        }
    }

    fn print_fo_arrow(
        &self,
        output: &mut impl Write,
        type_: &Type,
        problem_type: ProblemType,
    ) -> io::Result<()> {
        let nr_of_args = type_.arity() - 1;
        if nr_of_args == 1 {
            self.print_tstp(output, &type_.args()[0], problem_type)?;
            output.write_all(b" > ")?;
            self.print_tstp(output, &type_.args()[1], problem_type)?;
        } else {
            output.write_all(b"(")?;
            for arg in &type_.args()[..nr_of_args - 1] {
                self.print_tstp(output, arg, problem_type)?;
                output.write_all(b" * ")?;
            }
            self.print_tstp(output, &type_.args()[nr_of_args - 1], problem_type)?;
            output.write_all(b") > ")?;
            self.print_tstp(output, &type_.args()[type_.arity() - 1], problem_type)?;
        }
        Ok(())
    }

    fn print_ho_arrow(
        &self,
        output: &mut impl Write,
        type_: &Type,
        problem_type: ProblemType,
    ) -> io::Result<()> {
        for arg in &type_.args()[..type_.arity() - 1] {
            if arg.is_arrow() {
                output.write_all(b"(")?;
            }
            self.print_tstp(output, arg, problem_type)?;
            if arg.is_arrow() {
                output.write_all(b")")?;
            }
            output.write_all(b" > ")?;
        }
        self.print_tstp(output, &type_.args()[type_.arity() - 1], problem_type)
    }

    fn tc_name_or_io(&self, tc_code: TypeConsCode) -> io::Result<&str> {
        self.find_tc_name(tc_code).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unknown type constructor code {tc_code}"),
            )
        })
    }
}

fn ensure_not_kind(arg: &Type) -> Result<(), Diagnostic> {
    if arg.is_kind() {
        Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Only ground types supported.",
        ))
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TypeConstructorNameInfo {
    code: TypeConsCode,
    arity: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct TypeKey {
    f_code: TypeConsCode,
    arg_uids: Vec<TypeUniqueId>,
}

impl TypeKey {
    fn from_shared_type(type_: &Type) -> Self {
        debug_assert!(
            type_
                .args()
                .iter()
                .all(|arg| arg.type_uid() != INVALID_TYPE_UID),
            "compound type arguments must be shared before keying"
        );
        Self {
            f_code: type_.f_code(),
            arg_uids: type_.args().iter().map(Type::type_uid).collect(),
        }
    }
}

fn diagnostic_to_io(diagnostic: &Diagnostic) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, diagnostic.message().to_owned())
}

#[cfg(test)]
mod tests {
    use super::{TypeBank, NAME_NOT_FOUND, TYPEBANK_HASH_MASK, TYPEBANK_SIZE};
    use crate::basics::error::ErrorCode;
    use crate::basics::simple_stuff::ProblemType;
    use crate::inout::scanner::{Scanner, TokenType};
    use crate::terms::simpletypes::{
        alloc_arrow_type, alloc_simple_sort, sort_is_user_defined, type_alloc, ARROW_TYPE_CONS,
        INVALID_TYPE_UID, ST_BOOL, ST_INDIVIDUALS, ST_INTEGER, ST_KIND, ST_RATIONAL, ST_REAL,
    };

    fn scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, false).unwrap()
    }

    fn string_from(bytes: Vec<u8>) -> String {
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn constants_match_c_header() {
        assert_eq!(TYPEBANK_SIZE, 4096);
        assert_eq!(TYPEBANK_HASH_MASK, 4095);
        assert_eq!(NAME_NOT_FOUND, -1);
    }

    #[test]
    fn allocation_registers_predefined_constructors_and_types() {
        let bank = TypeBank::new();

        assert_eq!(bank.names_count(), 7);
        assert_eq!(bank.types_count(), 6);
        assert_eq!(bank.max_predefined_count(), 6);
        assert_eq!(bank.find_tc_code("$>_type"), ARROW_TYPE_CONS);
        assert_eq!(bank.find_tc_code("$o"), ST_BOOL);
        assert_eq!(bank.find_tc_code("$i"), ST_INDIVIDUALS);
        assert_eq!(bank.find_tc_code("$tType"), ST_KIND);
        assert_eq!(bank.find_tc_code("$int"), ST_INTEGER);
        assert_eq!(bank.find_tc_code("$rat"), ST_RATIONAL);
        assert_eq!(bank.find_tc_code("$real"), ST_REAL);
        assert_eq!(bank.find_tc_code("missing"), NAME_NOT_FOUND);
        assert_eq!(bank.find_tc_arity(ST_BOOL), Some(0));
        assert_eq!(bank.find_tc_name(ST_INTEGER), Some("$int"));

        assert_eq!(bank.bool_type().type_uid(), 1);
        assert_eq!(bank.i_type().type_uid(), 2);
        assert_eq!(bank.kind_type().type_uid(), 3);
        assert_eq!(bank.integer_type().type_uid(), 4);
        assert_eq!(bank.rational_type().type_uid(), 5);
        assert_eq!(bank.real_type().type_uid(), 6);
        assert_eq!(bank.default_type(), bank.i_type());
    }

    #[test]
    fn defining_type_constructors_preserves_names_and_rejects_arity_mismatch() {
        let mut bank = TypeBank::new();
        let list_code = bank.define_type_constructor("list", 1).unwrap();

        assert!(sort_is_user_defined(list_code));
        assert_eq!(bank.define_type_constructor("list", 1).unwrap(), list_code);
        assert_eq!(bank.find_tc_code("list"), list_code);
        assert_eq!(bank.find_tc_arity(list_code), Some(1));
        assert_eq!(bank.find_tc_name(list_code), Some("list"));

        let error = bank.define_type_constructor("list", 2).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);

        let sort_code = bank.define_simple_sort("person").unwrap();
        assert_eq!(bank.find_tc_arity(sort_code), Some(0));
    }

    #[test]
    fn insert_type_shared_assigns_uids_and_reuses_structural_matches() {
        let mut bank = TypeBank::new();
        let bool_a = alloc_simple_sort(ST_BOOL);
        let bool_b = bank.insert_type_shared(bool_a);

        assert_eq!(bool_b, bank.bool_type());
        assert_eq!(bank.types_count(), 6);

        let left = alloc_arrow_type(vec![
            alloc_simple_sort(ST_INDIVIDUALS),
            alloc_simple_sort(ST_BOOL),
        ]);
        let right = alloc_arrow_type(vec![
            alloc_simple_sort(ST_INDIVIDUALS),
            alloc_simple_sort(ST_BOOL),
        ]);
        assert_eq!(left.type_uid(), INVALID_TYPE_UID);
        assert_eq!(left.args()[0].type_uid(), INVALID_TYPE_UID);

        let shared_left = bank.insert_type_shared(left);
        let shared_right = bank.insert_type_shared(right);

        assert_eq!(shared_left, shared_right);
        assert_eq!(shared_left.type_uid(), 7);
        assert_eq!(shared_left.args()[0], bank.i_type());
        assert_eq!(shared_left.args()[1], bank.bool_type());
        assert_eq!(bank.types_count(), 7);
    }

    #[test]
    fn insert_type_shared_handles_user_type_constructors() {
        let mut bank = TypeBank::new();
        let list_code = bank.define_type_constructor("list", 1).unwrap();
        let list_of_int = type_alloc(list_code, vec![alloc_simple_sort(ST_INTEGER)]);
        let shared = bank.insert_type_shared(list_of_int);

        assert_eq!(shared.type_uid(), 7);
        assert_eq!(shared.args()[0], bank.integer_type());

        let mut output = Vec::new();
        bank.print_tstp(&mut output, &shared, ProblemType::HigherOrder)
            .unwrap();
        assert_eq!(string_from(output), "list($int)");
    }

    #[test]
    fn print_tstp_matches_fo_and_ho_arrow_shapes() {
        let mut bank = TypeBank::new();
        let unary = bank.insert_type_shared(alloc_arrow_type(vec![
            alloc_simple_sort(ST_INDIVIDUALS),
            alloc_simple_sort(ST_BOOL),
        ]));
        let binary = bank.insert_type_shared(alloc_arrow_type(vec![
            alloc_simple_sort(ST_INDIVIDUALS),
            alloc_simple_sort(ST_INTEGER),
            alloc_simple_sort(ST_BOOL),
        ]));
        let higher_arg = bank.insert_type_shared(alloc_arrow_type(vec![
            alloc_simple_sort(ST_INDIVIDUALS),
            alloc_simple_sort(ST_BOOL),
        ]));
        let higher = bank.insert_type_shared(alloc_arrow_type(vec![
            higher_arg,
            alloc_simple_sort(ST_BOOL),
        ]));

        let mut output = Vec::new();
        bank.print_tstp(&mut output, &unary, ProblemType::FirstOrder)
            .unwrap();
        assert_eq!(string_from(output), "$i > $o");

        let mut output = Vec::new();
        bank.print_tstp(&mut output, &binary, ProblemType::FirstOrder)
            .unwrap();
        assert_eq!(string_from(output), "($i * $int) > $o");

        let mut output = Vec::new();
        bank.print_tstp(&mut output, &higher, ProblemType::HigherOrder)
            .unwrap();
        assert_eq!(string_from(output), "($i > $o) > $o");
    }

    #[test]
    fn selected_sort_defs_print_only_user_simple_sorts() {
        let mut bank = TypeBank::new();
        let person_code = bank.define_simple_sort("person").unwrap();
        let animal_code = bank.define_simple_sort("animal").unwrap();
        let animal = bank.insert_type_shared(alloc_simple_sort(animal_code));
        let person = bank.insert_type_shared(alloc_simple_sort(person_code));
        let arrow = bank.insert_type_shared(alloc_arrow_type(vec![
            person.clone(),
            alloc_simple_sort(ST_BOOL),
        ]));
        let built_in = bank.i_type();
        let selected = [built_in, arrow, animal, person];

        let mut output = Vec::new();
        let count = bank
            .print_selected_sort_defs(&mut output, selected.iter(), ProblemType::HigherOrder)
            .unwrap();

        assert_eq!(count, 2);
        assert_eq!(
            string_from(output),
            "thf(decl_sort1, type, animal: $tType).\nthf(decl_sort2, type, person: $tType).\n"
        );
    }

    #[test]
    fn change_return_type_matches_c_shapes() {
        let mut bank = TypeBank::new();
        let predicate = bank.insert_type_shared(alloc_arrow_type(vec![
            alloc_simple_sort(ST_INDIVIDUALS),
            alloc_simple_sort(ST_BOOL),
        ]));

        let changed = bank.change_return_type(&predicate, &bank.integer_type());

        assert!(changed.is_arrow());
        assert_eq!(changed.args()[0], bank.i_type());
        assert_eq!(changed.args()[1], bank.integer_type());
        assert_eq!(
            bank.change_return_type(&bank.i_type(), &bank.integer_type()),
            bank.bool_type()
        );
    }

    #[test]
    fn app_encode_types_prints_arrow_and_user_sort_declarations() {
        let mut bank = TypeBank::new();
        let person_code = bank.define_simple_sort("person").unwrap();
        let person = bank.insert_type_shared(alloc_simple_sort(person_code));
        let arrow = bank.insert_type_shared(alloc_arrow_type(vec![
            person.clone(),
            alloc_simple_sort(ST_BOOL),
        ]));

        assert_eq!(person.type_uid(), 7);
        assert_eq!(arrow.type_uid(), 8);

        let mut output = Vec::new();
        let count = bank
            .app_encode_types(&mut output, ProblemType::HigherOrder, true)
            .unwrap();

        assert_eq!(count, 2);
        assert_eq!(
            string_from(output),
            "%-- person.\ntff(typedecl1, type, type_7: $tType).\n%-- person > $o.\ntff(typedecl2, type, type_8: $tType).\n"
        );
    }

    #[test]
    fn parse_type_reads_first_order_simple_and_arrow_types() {
        let mut bank = TypeBank::new();
        let mut input = scanner("$i. $i > $o. (($i * $int) > $o).");

        let individual = bank
            .parse_type(&mut input, ProblemType::FirstOrder)
            .unwrap();
        assert_eq!(individual, bank.i_type());
        input.accept_tok(TokenType::FULLSTOP).unwrap();

        let predicate = bank
            .parse_type(&mut input, ProblemType::FirstOrder)
            .unwrap();
        assert!(predicate.is_arrow());
        assert_eq!(predicate.args()[0], bank.i_type());
        assert_eq!(predicate.args()[1], bank.bool_type());
        input.accept_tok(TokenType::FULLSTOP).unwrap();

        let binary = bank
            .parse_type(&mut input, ProblemType::FirstOrder)
            .unwrap();
        assert_eq!(binary.arity(), 3);
        assert_eq!(binary.args()[0], bank.i_type());
        assert_eq!(binary.args()[1], bank.integer_type());
        assert_eq!(binary.args()[2], bank.bool_type());
        input.accept_tok(TokenType::FULLSTOP).unwrap();
    }

    #[test]
    fn parse_type_reads_type_constructor_applications_and_checks_arity() {
        let mut bank = TypeBank::new();
        let mut input = scanner("list($i). list($i). list($i, $o).");

        let list = bank
            .parse_type(&mut input, ProblemType::FirstOrder)
            .unwrap();
        input.accept_tok(TokenType::FULLSTOP).unwrap();
        let repeated = bank
            .parse_type(&mut input, ProblemType::FirstOrder)
            .unwrap();
        input.accept_tok(TokenType::FULLSTOP).unwrap();

        assert_eq!(list, repeated);
        assert_eq!(bank.find_tc_arity(bank.find_tc_code("list")), Some(1));
        assert_eq!(list.args()[0], bank.i_type());

        let error = bank
            .parse_type(&mut input, ProblemType::FirstOrder)
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Mismatch in number of arguments"));
    }

    #[test]
    fn parse_single_type_rejects_constructor_kind_arguments_and_sort_mismatch() {
        let mut bank = TypeBank::new();
        let mut input = scanner("list($tType). list.");
        let error = bank
            .parse_type(&mut input, ProblemType::FirstOrder)
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert_eq!(error.message(), "Only ground types supported.");

        let list_code = bank.define_type_constructor("list", 1).unwrap();
        assert!(sort_is_user_defined(list_code));
        let mut input = scanner("list.");
        let error = bank
            .parse_type(&mut input, ProblemType::FirstOrder)
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error
            .message()
            .contains("has not been declared as simple sort"));
    }

    #[test]
    fn parse_type_reads_higher_order_arrows_and_flattens_right_nesting() {
        let mut bank = TypeBank::new();
        let mut input = scanner("$i > $int > $o. ($i > $o) > $o.");

        let curried = bank
            .parse_type(&mut input, ProblemType::HigherOrder)
            .unwrap();
        assert_eq!(curried.arity(), 3);
        assert_eq!(curried.args()[0], bank.i_type());
        assert_eq!(curried.args()[1], bank.integer_type());
        assert_eq!(curried.args()[2], bank.bool_type());
        input.accept_tok(TokenType::FULLSTOP).unwrap();

        let higher = bank
            .parse_type(&mut input, ProblemType::HigherOrder)
            .unwrap();
        assert_eq!(higher.arity(), 2);
        assert!(higher.args()[0].is_arrow());
        assert_eq!(higher.args()[1], bank.bool_type());
        input.accept_tok(TokenType::FULLSTOP).unwrap();
    }

    #[test]
    fn parse_type_reports_higher_order_mixed_syntax_and_missing_problem_type() {
        let mut bank = TypeBank::new();
        let mut input = scanner("$i * $o.");
        let error = bank
            .parse_type(&mut input, ProblemType::HigherOrder)
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Mixing of first order"));

        let mut input = scanner("$i.");
        let error = bank
            .parse_type(&mut input, ProblemType::NotInitialized)
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
    }
}
