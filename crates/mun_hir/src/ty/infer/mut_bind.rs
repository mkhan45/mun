use crate::resolve::ValueNs;
use crate::{
    expr::BindingAnnotation, ty::infer::InferenceResultBuilder, Expr, ExprId, Pat, Path, Resolver,
};
use std::sync::Arc;

impl<'a> InferenceResultBuilder<'a> {
    /// Checks if the specified expression is a place-expression. A place expression represents a
    /// memory location.
    pub(super) fn check_mut_bind(&mut self, resolver: &Resolver, expr: ExprId) -> bool {
        let body = Arc::clone(&self.body); // avoid borrow checker problem
        match &body[expr] {
            Expr::Path(p) => self.check_mut_path(resolver, p),
            Expr::Field { expr, .. } => self.check_mut_bind(resolver, *expr),
            _ => false,
        }
    }

    /// Checks if the specified path references a mutable variable
    fn check_mut_path(&mut self, resolver: &Resolver, path: &Path) -> bool {
        let body = Arc::clone(&self.body); // avoid borrow checker problem
        if let Some((ValueNs::LocalBinding(pat), _)) =
            resolver.resolve_path_as_value_fully(self.db.upcast(), path)
        {
            matches!(body[pat], Pat::Bind{ mode: BindingAnnotation::Mutable, .. })
        } else {
            false
        }
    }
}
