use crate::basics::error::{Diagnostic, ErrorCode};
use crate::heuristics::hcb::{hcb_add_wfcb, hcb_alloc, HcbCell, HCB_DEFAULT_HEURISTIC};
use crate::heuristics::wfcbadmin::{WeightParseContext, WfcbAdmin};
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};

#[derive(Default)]
pub struct HcbAdmin {
    entries: Vec<HcbAdminEntry>,
}

struct HcbAdminEntry {
    name: String,
    hcb: HcbCell<()>,
}

impl HcbAdmin {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn name(&self, index: usize) -> Option<&str> {
        self.entries.get(index).map(|entry| entry.name.as_str())
    }

    #[must_use]
    pub fn hcb(&self, index: usize) -> Option<&HcbCell<()>> {
        self.entries.get(index).map(|entry| &entry.hcb)
    }

    pub fn hcb_mut(&mut self, index: usize) -> Option<&mut HcbCell<()>> {
        self.entries.get_mut(index).map(|entry| &mut entry.hcb)
    }

    pub fn add_hcb(&mut self, name: impl Into<String>, hcb: HcbCell<()>) -> usize {
        self.entries.push(HcbAdminEntry {
            name: name.into(),
            hcb,
        });
        self.entries.len() - 1
    }

    #[must_use]
    pub fn find_hcb(&self, name: &str) -> Option<&HcbCell<()>> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.hcb)
    }

    pub fn find_hcb_mut(&mut self, name: &str) -> Option<&mut HcbCell<()>> {
        self.entries
            .iter_mut()
            .rev()
            .find(|entry| entry.name == name)
            .map(|entry| &mut entry.hcb)
    }

    pub fn heuristic_def_parse(
        &mut self,
        scanner: &mut Scanner,
        wfcbs: &mut WfcbAdmin,
    ) -> Result<usize, Diagnostic> {
        self.heuristic_def_parse_with_context(scanner, wfcbs, WeightParseContext::empty())
    }

    pub fn heuristic_def_parse_with_context(
        &mut self,
        scanner: &mut Scanner,
        wfcbs: &mut WfcbAdmin,
        context: WeightParseContext<'_>,
    ) -> Result<usize, Diagnostic> {
        let name = if scanner.test_tok(TokenType::OPEN_BRACKET) {
            HCB_DEFAULT_HEURISTIC.to_owned()
        } else {
            scanner.check_tok(TokenType::IDENTIFIER)?;
            let name = scanner.current_token().literal();
            scanner.next_token()?;
            scanner.accept_tok(TokenType::EQUAL_SIGN)?;
            name
        };
        let hcb = heuristic_parse_with_context(scanner, wfcbs, context)?;
        Ok(self.add_hcb(name, hcb))
    }

    pub fn heuristic_def_list_parse(
        &mut self,
        scanner: &mut Scanner,
        wfcbs: &mut WfcbAdmin,
    ) -> Result<usize, Diagnostic> {
        self.heuristic_def_list_parse_with_context(scanner, wfcbs, WeightParseContext::empty())
    }

    pub fn heuristic_def_list_parse_with_context(
        &mut self,
        scanner: &mut Scanner,
        wfcbs: &mut WfcbAdmin,
        context: WeightParseContext<'_>,
    ) -> Result<usize, Diagnostic> {
        let mut result = self.len();
        while (scanner.test_tok(TokenType::IDENTIFIER)
            && scanner
                .look_token(1)
                .kind()
                .intersects(TokenType::EQUAL_SIGN))
            || scanner.test_tok(TokenType::OPEN_BRACKET)
        {
            result = self.heuristic_def_parse_with_context(scanner, wfcbs, context)?;
        }
        Ok(result)
    }
}

#[must_use]
pub const fn hcb_admin_alloc() -> HcbAdmin {
    HcbAdmin::new()
}

pub fn hcb_admin_add_hcb(set: &mut HcbAdmin, name: impl Into<String>, hcb: HcbCell<()>) -> usize {
    set.add_hcb(name, hcb)
}

#[must_use]
pub fn hcb_admin_find_hcb<'a>(set: &'a HcbAdmin, name: &str) -> Option<&'a HcbCell<()>> {
    set.find_hcb(name)
}

pub fn heuristic_parse(
    scanner: &mut Scanner,
    wfcbs: &mut WfcbAdmin,
) -> Result<HcbCell<()>, Diagnostic> {
    heuristic_parse_with_context(scanner, wfcbs, WeightParseContext::empty())
}

pub fn heuristic_parse_with_context(
    scanner: &mut Scanner,
    wfcbs: &mut WfcbAdmin,
    context: WeightParseContext<'_>,
) -> Result<HcbCell<()>, Diagnostic> {
    let mut hcb = hcb_alloc();

    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    parse_single_wfcb_item(&mut hcb, scanner, wfcbs, context)?;
    while scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        parse_single_wfcb_item(&mut hcb, scanner, wfcbs, context)?;
    }
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok(hcb)
}

pub fn heuristic_def_parse(
    set: &mut HcbAdmin,
    scanner: &mut Scanner,
    wfcbs: &mut WfcbAdmin,
) -> Result<usize, Diagnostic> {
    set.heuristic_def_parse(scanner, wfcbs)
}

pub fn heuristic_def_parse_with_context(
    set: &mut HcbAdmin,
    scanner: &mut Scanner,
    wfcbs: &mut WfcbAdmin,
    context: WeightParseContext<'_>,
) -> Result<usize, Diagnostic> {
    set.heuristic_def_parse_with_context(scanner, wfcbs, context)
}

pub fn heuristic_def_list_parse(
    set: &mut HcbAdmin,
    scanner: &mut Scanner,
    wfcbs: &mut WfcbAdmin,
) -> Result<usize, Diagnostic> {
    set.heuristic_def_list_parse(scanner, wfcbs)
}

pub fn heuristic_def_list_parse_with_context(
    set: &mut HcbAdmin,
    scanner: &mut Scanner,
    wfcbs: &mut WfcbAdmin,
    context: WeightParseContext<'_>,
) -> Result<usize, Diagnostic> {
    set.heuristic_def_list_parse_with_context(scanner, wfcbs, context)
}

fn parse_single_wfcb_item(
    hcb: &mut HcbCell<()>,
    scanner: &mut Scanner,
    wfcbs: &mut WfcbAdmin,
    context: WeightParseContext<'_>,
) -> Result<(), Diagnostic> {
    scanner.check_tok(TokenType::POS_INT)?;
    let steps = i64::try_from(scanner.current_token().numval()).map_err(|_| {
        hcb_admin_error(
            scanner,
            "Value >0 expected in heuristic evaluation function description",
        )
    })?;
    if steps <= 0 {
        return Err(hcb_admin_error(
            scanner,
            "Value >0 expected in heuristic evaluation function description",
        ));
    }
    scanner.accept_tok(TokenType::POS_INT)?;
    scanner.accept_tok(TokenType::MULT | TokenType::FULLSTOP)?;
    scanner.check_tok(TokenType::IDENTIFIER)?;

    let handle = if scanner
        .look_token(1)
        .kind()
        .intersects(TokenType::OPEN_BRACKET | TokenType::EQUAL_SIGN)
    {
        let name = wfcbs.weight_fun_def_parse_with_context(scanner, context)?;
        wfcbs.find_wfcb_handle(&name)
    } else {
        let name = scanner.current_token().literal();
        let handle = wfcbs.find_wfcb_handle(&name);
        scanner.next_token()?;
        handle
    };

    let Some(handle) = handle else {
        return Err(hcb_admin_error(
            scanner,
            "Not a valid evaluation function specifier",
        ));
    };
    hcb_add_wfcb(hcb, handle, steps);
    Ok(())
}

fn hcb_admin_error(scanner: &Scanner, message: &str) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!(
            "{}(just read '{}'): {message}",
            token_pos_rep(scanner.current_token()),
            scanner.current_token().literal()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        hcb_admin_add_hcb, hcb_admin_alloc, hcb_admin_find_hcb, heuristic_def_list_parse,
        heuristic_def_parse, heuristic_parse, HcbAdmin,
    };
    use crate::heuristics::wfcbadmin::{weight_fun_parse, WfcbAdmin};
    use crate::inout::scanner::Scanner;

    fn scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, false).unwrap_or_else(|err| panic!("{err}"))
    }

    fn add_fifo(admin: &mut WfcbAdmin, name: &str) -> usize {
        let mut scanner = scanner("FIFOWeight(ConstPrio)");
        admin.add_wfcb(
            name,
            weight_fun_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}")),
        )
    }

    fn add_lifo(admin: &mut WfcbAdmin, name: &str) -> usize {
        let mut scanner = scanner("LIFOWeight(ConstPrio)");
        admin.add_wfcb(
            name,
            weight_fun_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}")),
        )
    }

    #[test]
    fn allocation_starts_empty_like_c_admin_cell() {
        let admin = hcb_admin_alloc();

        assert_eq!(admin.len(), 0);
        assert!(admin.is_empty());
        assert_eq!(admin.name(0), None);
        assert!(admin.hcb(0).is_none());
    }

    #[test]
    fn add_and_find_return_last_duplicate_hcb_name() {
        let mut admin = HcbAdmin::new();
        let first = hcb_admin_add_hcb(&mut admin, "Default", crate::heuristics::hcb::hcb_alloc());
        let second = hcb_admin_add_hcb(&mut admin, "Other", crate::heuristics::hcb::hcb_alloc());
        let mut replacement = crate::heuristics::hcb::hcb_alloc();
        crate::heuristics::hcb::hcb_add_wfcb(&mut replacement, 99, 1);
        let third = hcb_admin_add_hcb(&mut admin, "Default", replacement);

        assert_eq!(first, 0);
        assert_eq!(second, 1);
        assert_eq!(third, 2);
        assert_eq!(admin.name(first), Some("Default"));
        assert_eq!(admin.name(second), Some("Other"));
        assert_eq!(
            hcb_admin_find_hcb(&admin, "Default").map(crate::heuristics::hcb::HcbCell::wfcb_no),
            Some(1)
        );
        assert!(admin.find_hcb("Missing").is_none());
        assert!(admin.hcb_mut(second).is_some());
        assert!(admin.find_hcb_mut("Other").is_some());
    }

    #[test]
    fn heuristic_parse_accepts_named_wfcbs_and_both_separators() {
        let mut wfcbs = WfcbAdmin::new();
        let fifo = add_fifo(&mut wfcbs, "fifo");
        let lifo = add_lifo(&mut wfcbs, "lifo");
        let mut scanner = scanner("(2*fifo,3.lifo) tail");

        let hcb = heuristic_parse(&mut scanner, &mut wfcbs).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(hcb.wfcb_no(), 2);
        assert_eq!(hcb.wfcb_handle(0), Some(fifo));
        assert_eq!(hcb.wfcb_handle(1), Some(lifo));
        assert_eq!(hcb.select_switch(0), Some(2));
        assert_eq!(hcb.select_switch(1), Some(5));
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn heuristic_parse_adds_inline_anonymous_and_named_weight_defs() {
        let mut wfcbs = WfcbAdmin::new();
        let mut scanner =
            scanner("(1*FIFOWeight(ConstPrio),2*local=LIFOWeight(ConstPrio),3*local) tail");

        let hcb = heuristic_parse(&mut scanner, &mut wfcbs).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(wfcbs.len(), 2);
        assert_eq!(wfcbs.name(0), Some("~$000000000"));
        assert_eq!(wfcbs.name(1), Some("local"));
        assert_eq!(wfcbs.anon_counter(), 1);
        assert_eq!(hcb.wfcb_handle(0), Some(0));
        assert_eq!(hcb.wfcb_handle(1), Some(1));
        assert_eq!(hcb.wfcb_handle(2), Some(1));
        assert_eq!(hcb.select_switch(0), Some(1));
        assert_eq!(hcb.select_switch(1), Some(3));
        assert_eq!(hcb.select_switch(2), Some(6));
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn heuristic_def_parse_uses_default_name_for_bare_bracket() {
        let mut wfcbs = WfcbAdmin::new();
        let fifo = add_fifo(&mut wfcbs, "fifo");
        let mut hcb_admin = HcbAdmin::new();
        let mut scanner = scanner("(1*fifo) tail");

        let index = heuristic_def_parse(&mut hcb_admin, &mut scanner, &mut wfcbs)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(index, 0);
        assert_eq!(hcb_admin.name(index), Some("Default"));
        assert_eq!(
            hcb_admin.hcb(index).and_then(|hcb| hcb.wfcb_handle(0)),
            Some(fifo)
        );
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn heuristic_def_list_parse_returns_last_index_not_count() {
        let mut wfcbs = WfcbAdmin::new();
        add_fifo(&mut wfcbs, "fifo");
        let mut hcb_admin = HcbAdmin::new();
        hcb_admin.add_hcb("Existing", crate::heuristics::hcb::hcb_alloc());
        let mut definitions = scanner("First=(1*fifo) Default=(2*fifo) done");

        let result = heuristic_def_list_parse(&mut hcb_admin, &mut definitions, &mut wfcbs)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(result, 2);
        assert_eq!(hcb_admin.len(), 3);
        assert_eq!(hcb_admin.name(1), Some("First"));
        assert_eq!(hcb_admin.name(2), Some("Default"));
        assert_eq!(
            hcb_admin
                .find_hcb("Default")
                .and_then(|hcb| hcb.select_switch(0)),
            Some(2)
        );
        assert_eq!(definitions.current_token().literal(), "done");

        let mut empty_list = scanner("done");
        let result = heuristic_def_list_parse(&mut hcb_admin, &mut empty_list, &mut wfcbs)
            .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(result, 3);
        assert_eq!(empty_list.current_token().literal(), "done");
    }

    #[test]
    fn heuristic_parse_rejects_nonpositive_steps_and_unknown_wfcb_names() {
        let mut wfcbs = WfcbAdmin::new();
        let mut zero = scanner("(0*missing)");
        let Err(error) = heuristic_parse(&mut zero, &mut wfcbs) else {
            panic!("zero-step heuristic item should fail");
        };
        assert!(error
            .message()
            .contains("Value >0 expected in heuristic evaluation function description"));

        let mut missing = scanner("(1*missing)");
        let Err(error) = heuristic_parse(&mut missing, &mut wfcbs) else {
            panic!("unknown WFCB name should fail");
        };
        assert!(error
            .message()
            .contains("Not a valid evaluation function specifier"));
    }
}
