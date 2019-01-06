// Copyright (c) 2016-2019 Fabian Schuiki

//! A mapping from node IDs to AST nodes.

use crate::ast;
use crate::common::source::Span;
use crate::common::util::{HasDesc, HasSpan};
use crate::common::NodeId;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Default)]
pub struct AstMap<'ast> {
    map: RefCell<HashMap<NodeId, AstNode<'ast>>>,
}

impl<'ast> AstMap<'ast> {
    /// Insert an AST node into the map.
    pub fn set(&self, id: NodeId, node: impl Into<AstNode<'ast>>) {
        let node = node.into();
        if self.map.borrow_mut().insert(id, node).is_some() {
            panic!("node {:?} already exists in the map", id);
        }
    }

    /// Retrieve an AST node from the map.
    pub fn get(&self, id: NodeId) -> Option<AstNode<'ast>> {
        self.map.borrow().get(&id).cloned()
    }
}

/// A reference to an AST node.
///
/// This enum essentially provides a wrapper around typed references to AST
/// nodes. It allows code to obtain a generic reference to an AST node and then
/// match on the actual type that was provided.
#[derive(Clone, Copy, Debug)]
pub enum AstNode<'ast> {
    Module(&'ast ast::ModDecl),
    Port(&'ast ast::Port),
    Type(&'ast ast::Type),
    /// The module/interface name and parameters of an instantiation.
    InstTarget(&'ast ast::Inst),
    /// An instance name and reference to its target.
    Inst(&'ast ast::InstName, NodeId),
    /// A type parameter.
    TypeParam(&'ast ast::ParamDecl, &'ast ast::ParamTypeDecl),
    /// A value parameter.
    ValueParam(&'ast ast::ParamDecl, &'ast ast::ParamValueDecl),
}

impl<'ast> HasSpan for AstNode<'ast> {
    fn span(&self) -> Span {
        match *self {
            AstNode::Module(x) => x.span(),
            AstNode::Port(x) => x.span(),
            AstNode::Type(x) => x.span(),
            AstNode::InstTarget(x) => x.span(),
            AstNode::Inst(x, _) => x.span(),
            AstNode::TypeParam(x, _) => x.span(),
            AstNode::ValueParam(x, _) => x.span(),
        }
    }

    fn human_span(&self) -> Span {
        match *self {
            AstNode::Module(x) => x.human_span(),
            AstNode::Port(x) => x.human_span(),
            AstNode::Type(x) => x.human_span(),
            AstNode::InstTarget(x) => x.human_span(),
            AstNode::Inst(x, _) => x.human_span(),
            AstNode::TypeParam(_, x) => x.human_span(),
            AstNode::ValueParam(_, x) => x.human_span(),
        }
    }
}

impl<'ast> HasDesc for AstNode<'ast> {
    fn desc(&self) -> &'static str {
        match *self {
            AstNode::Module(x) => x.desc(),
            AstNode::Port(x) => x.desc(),
            AstNode::Type(x) => x.desc(),
            AstNode::InstTarget(x) => x.desc(),
            AstNode::Inst(x, _) => x.desc(),
            AstNode::TypeParam(_, x) => x.desc(),
            AstNode::ValueParam(_, x) => x.desc(),
        }
    }

    fn desc_full(&self) -> String {
        match *self {
            AstNode::Module(x) => x.desc_full(),
            AstNode::Port(x) => x.desc_full(),
            AstNode::Type(x) => x.desc_full(),
            AstNode::InstTarget(x) => x.desc_full(),
            AstNode::Inst(x, _) => x.desc_full(),
            AstNode::TypeParam(_, x) => x.desc_full(),
            AstNode::ValueParam(_, x) => x.desc_full(),
        }
    }
}
