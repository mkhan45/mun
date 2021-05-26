use super::*;

pub(super) fn struct_def(p: &mut Parser, m: Marker) {
    assert!(p.at(T![struct]));
    p.bump(T![struct]);
    opt_memory_type_specifier(p);
    name_recovery(p, declarations::DECLARATION_RECOVERY_SET);
    match p.current() {
        T![;] => {
            p.bump(T![;]);
        }
        T!['{'] => record_field_def_list(p),
        T!['('] => tuple_field_def_list(p),
        _ => {
            p.error("expected a ';', '{', or '('");
        }
    }
    m.complete(p, STRUCT_DEF);
}

pub(super) fn type_alias_def(p: &mut Parser, m: Marker) {
    assert!(p.at(T![type]));
    p.bump(T![type]);
    name(p);
    if p.eat(T![=]) {
        types::type_(p);
    }
    p.expect(T![;]);
    m.complete(p, TYPE_ALIAS_DEF);
}

pub(super) fn const_def(p: &mut Parser, m: Marker) {
    assert!(p.eat(T![const]));
    name(p);
    if p.at(T![:]) {
        types::ascription(p);
    }
    p.expect(T![=]);
    expressions::expr(p);

    p.expect(T![;]);
    m.complete(p, CONST_DEF);
}

pub(super) fn record_field_def_list(p: &mut Parser) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    while !p.at(T!['}']) && !p.at(EOF) {
        if p.at(T!['{']) {
            error_block(p, "expected a field");
            continue;
        }
        record_field_def(p);
        if !p.at(T!['}']) {
            p.expect(T![,]);
        }
    }
    p.expect(T!['}']);
    p.eat(T![;]);
    m.complete(p, RECORD_FIELD_DEF_LIST);
}

fn opt_memory_type_specifier(p: &mut Parser) {
    if p.at(T!['(']) {
        let m = p.start();
        p.bump(T!['(']);
        if p.at(IDENT) {
            if p.at_contextual_kw("gc") {
                p.bump_remap(GC_KW)
            } else if p.at_contextual_kw("value") {
                p.bump_remap(VALUE_KW)
            } else {
                p.error_and_bump("expected memory type specifier");
            }
        } else {
            p.error("expected memory type specifier");
        }
        p.expect(T![')']);
        m.complete(p, MEMORY_TYPE_SPECIFIER);
    }
}

pub(super) fn tuple_field_def_list(p: &mut Parser) {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    while !p.at(T![')']) && !p.at(EOF) {
        let m = p.start();
        if !p.at_ts(types::TYPE_FIRST) {
            m.abandon(p);
            p.error_and_bump("expected a type");
            break;
        }
        types::type_(p);
        m.complete(p, TUPLE_FIELD_DEF);

        if !p.at(T![')']) {
            p.expect(T![,]);
        }
    }
    p.expect(T![')']);
    p.eat(T![;]);
    m.complete(p, TUPLE_FIELD_DEF_LIST);
}

fn record_field_def(p: &mut Parser) {
    let m = p.start();
    opt_visibility(p);
    if p.at(IDENT) {
        name(p);
        p.expect(T![:]);
        types::type_(p);
        m.complete(p, RECORD_FIELD_DEF);
    } else {
        m.abandon(p);
        p.error_and_bump("expected a field declaration");
    }
}
