// Copyright (c) 2016-2020 Fabian Schuiki

//! A parameter environment generated by an instantiation.

use crate::{
    ast_map::AstNode,
    crate_prelude::*,
    hir::{NamedParam, PosParam},
    ty::UnpackedType,
    value::Value,
};

/// A parameter environment.
///
/// This is merely an handle that is cheap to copy and pass around. Use the
/// [`Context`] to resolve this to the actual [`ParamEnvData`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParamEnv(pub(crate) u32);

impl std::fmt::Display for ParamEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "p{}", self.0)
    }
}

impl std::fmt::Debug for ParamEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// A node id with corresponding parameter environment.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeEnvId(NodeId, ParamEnv);

impl NodeEnvId {
    /// Create a new combined node ID with associated parameter bindings.
    pub fn new(id: NodeId, env: ParamEnv) -> Self {
        NodeEnvId(id, env)
    }

    /// Obtain the underlying node.
    pub fn id(self) -> NodeId {
        self.0
    }

    /// Obtain the parameter bindings associated with the node.
    pub fn env(self) -> ParamEnv {
        self.1
    }
}

/// A helper trait to allow for easy wrapping of node IDs.
pub trait IntoNodeEnvId {
    /// Associate parameter bindings with this node.
    fn env(self, env: ParamEnv) -> NodeEnvId;
}

impl IntoNodeEnvId for NodeId {
    fn env(self, env: ParamEnv) -> NodeEnvId {
        NodeEnvId(self, env)
    }
}

impl std::fmt::Display for NodeEnvId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}@{}", self.0, self.1)
    }
}

impl std::fmt::Debug for NodeEnvId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// A parameter environment.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ParamEnvData<'t> {
    module: Option<NodeId>,
    values: Vec<(NodeId, ParamEnvBinding<Value<'t>>)>,
    types: Vec<(NodeId, ParamEnvBinding<&'t UnpackedType<'t>>)>,
    intfs: Vec<(NodeId, NodeEnvId)>,
}

impl<'t> ParamEnvData<'t> {
    /// Find the value assigned to a node.
    pub fn find_value(&self, node_id: NodeId) -> Option<ParamEnvBinding<Value<'t>>> {
        self.values
            .iter()
            .find(|&&(id, _)| id == node_id)
            .map(|&(_, id)| id)
    }

    /// Find the type assigned to a node.
    pub fn find_type(&self, node_id: NodeId) -> Option<ParamEnvBinding<&'t UnpackedType<'t>>> {
        self.types
            .iter()
            .find(|&&(id, _)| id == node_id)
            .map(|&(_, id)| id)
    }

    /// Find the parametrization of an interface port.
    pub fn find_interface(&self, node_id: NodeId) -> Option<NodeEnvId> {
        self.intfs
            .iter()
            .find(|&&(id, _)| id == node_id)
            .map(|&(_, id)| id)
    }

    /// Find the node assigned to a value parameter.
    pub fn reverse_find_value(&self, node_id: NodeId) -> Option<NodeId> {
        self.values
            .iter()
            .flat_map(|&(param_id, binding)| match binding {
                ParamEnvBinding::Indirect(bound_id) if bound_id.id() == node_id => Some(param_id),
                _ => None,
            })
            .next()
    }

    /// Assign a value to a node.
    pub fn set_value(&mut self, node_id: NodeId, value: Value<'t>) {
        self.values.retain(|&(n, _)| n != node_id);
        self.values.push((node_id, ParamEnvBinding::Direct(value)));
    }

    /// Add additional interface parametrizations.
    pub fn add_interfaces(&mut self, iter: impl IntoIterator<Item = (NodeId, NodeEnvId)>) {
        self.intfs.extend(iter);
    }
}

/// A binding in a parameter environment.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ParamEnvBinding<T> {
    /// A direct binding, directly assigning a type or value to a node.
    Direct(T),
    /// An indirect binding, pointing at another node's type or value.
    Indirect(NodeEnvId),
}

/// A location that implies a parameter environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamEnvSource<'hir> {
    ModuleInst {
        module: Ref<'hir, hir::Module<'hir>>,
        env: ParamEnv,
        pos: &'hir [PosParam],
        named: &'hir [NamedParam],
    },
    InterfaceInst {
        interface: Ref<'hir, hir::Interface<'hir>>,
        env: ParamEnv,
        pos: &'hir [PosParam],
        named: &'hir [NamedParam],
    },
}

pub(crate) fn compute<'gcx>(
    cx: &impl Context<'gcx>,
    src: ParamEnvSource<'gcx>,
) -> Result<ParamEnv> {
    match src {
        ParamEnvSource::ModuleInst {
            module,
            env,
            pos,
            named,
        } => param_env_from_instance(
            cx,
            module.ast,
            module
                .params
                .iter()
                .cloned()
                .chain(module.block.params.iter().cloned())
                .collect(),
            env,
            pos,
            named,
        ),
        ParamEnvSource::InterfaceInst {
            interface,
            env,
            pos,
            named,
        } => param_env_from_instance(
            cx,
            interface.ast,
            interface
                .params
                .iter()
                .flat_map(|p| match &p.kind {
                    ast::ParamKind::Type(x) => {
                        x.iter().map(|d| d.id()).collect::<Vec<_>>().into_iter()
                    }
                    ast::ParamKind::Value(x) => {
                        x.iter().map(|d| d.id()).collect::<Vec<_>>().into_iter()
                    }
                })
                .chain(interface.block.params.iter().cloned())
                .collect(),
            env,
            pos,
            named,
        ),
    }
}

fn param_env_from_instance<'a>(
    cx: &impl Context<'a>,
    node: &'a dyn ast::AnyNode<'a>,
    params: Vec<NodeId>,
    env: ParamEnv,
    pos: &[PosParam],
    named: &[NamedParam],
) -> Result<ParamEnv> {
    // Associate the positional and named assignments with the actual
    // parameters of the module.
    let param_iter = pos
        .iter()
        .enumerate()
        .map(|(index, &(span, assign_id))| match params.get(index) {
            Some(&param_id) => Ok((param_id, (assign_id, env))),
            None => {
                cx.emit(
                    DiagBuilder2::error(format!("{} only has {} parameter(s)", node, params.len()))
                        .span(span),
                );
                Err(())
            }
        })
        .chain(named.iter().map(|&(_span, name, assign_id)| {
            let names: Vec<_> = params
                .iter()
                .flat_map(|&id| match cx.ast_of(id) {
                    Ok(AstNode::TypeParam(_, p)) => Some((p.name.value, id)),
                    Ok(AstNode::ValueParam(_, p)) => Some((p.name.value, id)),
                    Ok(_) => unreachable!(),
                    Err(()) => None,
                })
                .collect();
            match names
                .iter()
                .find(|&(param_name, _)| *param_name == name.value)
            {
                Some(&(_, param_id)) => Ok((param_id, (assign_id, env))),
                None => {
                    cx.emit(
                        DiagBuilder2::error(format!("no parameter `{}` in {}", name, node,))
                            .span(name.span)
                            .add_note(format!(
                                "declared parameters are {}",
                                names
                                    .iter()
                                    .map(|&(n, _)| format!("`{}`", n))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )),
                    );
                    Err(())
                }
            }
        }));
    let param_iter = param_iter
        .collect::<Vec<_>>()
        .into_iter()
        .collect::<Result<Vec<_>>>()?
        .into_iter();

    // Split up type and value parameters.
    let mut types = vec![];
    let mut values = vec![];
    for (param_id, assign_id) in param_iter {
        let assign_id = match assign_id {
            (Some(i), n) => i.env(n),
            _ => continue,
        };
        match cx.ast_of(param_id)? {
            AstNode::TypeParam(..) => {
                cx.set_lowering_hint(assign_id.0, hir::Hint::Type);
                types.push((param_id, ParamEnvBinding::Indirect(assign_id)))
            }
            AstNode::ValueParam(..) => {
                cx.set_lowering_hint(assign_id.0, hir::Hint::Expr);
                values.push((param_id, ParamEnvBinding::Indirect(assign_id)))
            }
            _ => unreachable!(),
        }
    }

    let env = cx.intern_param_env(ParamEnvData {
        module: Some(node.id()),
        types,
        values,
        intfs: Default::default(),
    });
    cx.add_param_env_context(env, node.id());
    Ok(env)
}
