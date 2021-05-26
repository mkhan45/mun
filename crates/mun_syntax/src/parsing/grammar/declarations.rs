use super::*;
use crate::T;

pub(super) const DECLARATION_RECOVERY_SET: TokenSet = TokenSet::new(&[T![fn], T![pub], T![struct]]);

pub(super) fn mod_contents(p: &mut Parser) {
    while !p.at(EOF) {
        declaration(p);
    }
}

pub(super) fn declaration(p: &mut Parser) {
    let m = p.start();
    let m = match maybe_declaration(p, m) {
        Ok(()) => return,
        Err(m) => m,
    };

    m.abandon(p);
    if p.at(T!['{']) {
        error_block(p, "expected a declaration")
    } else if p.at(T!['}']) {
        let e = p.start();
        p.error("unmatched }");
        p.bump(T!['}']);
        e.complete(p, ERROR);
    } else if !p.at(EOF) {
        p.error_and_bump("expected a declaration");
    } else {
        p.error("expected a declaration");
    }
}

pub(super) fn maybe_declaration(p: &mut Parser, m: Marker) -> Result<(), Marker> {
    opt_visibility(p);

    let m = match declarations_without_modifiers(p, m) {
        Ok(()) => return Ok(()),
        Err(m) => m,
    };

    if p.at(T![extern]) {
        abi(p);
    }

    match p.current() {
        T![fn] => {
            fn_def(p);
            m.complete(p, FUNCTION_DEF);
        }
        _ => return Err(m),
    }
    Ok(())
}

fn abi(p: &mut Parser) {
    assert!(p.at(T![extern]));
    let abi = p.start();
    p.bump(T![extern]);
    abi.complete(p, EXTERN);
}

fn declarations_without_modifiers(p: &mut Parser, m: Marker) -> Result<(), Marker> {
    match p.current() {
        T![struct] => {
            adt::struct_def(p, m);
        }
        T![type] => {
            adt::type_alias_def(p, m);
        }
        T![const] => {
            adt::const_def(p, m);
        }
        _ => return Err(m),
    };
    Ok(())
}

pub(super) fn fn_def(p: &mut Parser) {
    assert!(p.at(T![fn]));
    p.bump(T![fn]);

    name_recovery(p, DECLARATION_RECOVERY_SET.union(TokenSet::new(&[T![')']])));

    if p.at(T!['(']) {
        params::param_list(p);
    } else {
        p.error("expected function arguments")
    }

    opt_fn_ret_type(p);

    if p.at(T![;]) {
        p.bump(T![;]);
    } else {
        expressions::block(p);
    }
}

fn opt_fn_ret_type(p: &mut Parser) -> bool {
    if p.at(T![->]) {
        let m = p.start();
        p.bump(T![->]);
        types::type_(p);
        m.complete(p, RET_TYPE);
        true
    } else {
        false
    }
}
