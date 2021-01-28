use super::*;

pub(super) const PATTERN_FIRST: TokenSet = expressions::LITERAL_FIRST
    .union(paths::PATH_FIRST)
    .union(TokenSet::new(&[T![-], T![_], T![mut]]));

pub(super) fn pattern(p: &mut Parser) {
    pattern_r(p, PATTERN_FIRST);
}

pub(super) fn pattern_r(p: &mut Parser, recovery_set: TokenSet) {
    atom_pat(p, recovery_set);
}

fn atom_pat(p: &mut Parser, recovery_set: TokenSet) -> Option<CompletedMarker> {
    let m = match p.nth(0) {
        IDENT | T![mut] => bind_pat(p),
        T![_] => placeholder_pat(p),
        _ => {
            p.error_recover("expected pattern", recovery_set);
            return None;
        }
    };

    Some(m)
}

fn placeholder_pat(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![_]));
    let m = p.start();
    p.bump(T![_]);
    m.complete(p, PLACEHOLDER_PAT)
}

fn bind_pat(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    p.eat(T![mut]);
    name(p);
    m.complete(p, BIND_PAT)
}
