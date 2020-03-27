use crate::{
    context::{Context, Symbol, Term},
    lr::Action,
};
use anyhow::Result;
use heck::{CamelCase, TitleCase};
use itertools::Itertools;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    io::Write,
};

/// Generate the parser code.
pub fn codegen(ctx: &mut Context, into: &mut impl Write) -> Result<()> {
    write!(into, "// Automatically generated by pargen\n")?;

    // Build a table of state predecessors to decide whether to inline states
    // or not.
    let mut preds = HashMap::<_, HashSet<_>>::new();
    let mut invariant_states = HashSet::new();
    for (&state, actions) in &ctx.lr_table.actions {
        let mut unified_actions = HashSet::new();
        for (_, actions) in actions {
            for &action in actions {
                match action {
                    Action::Shift(s) => {
                        preds.entry(s).or_default().insert(state);
                    }
                    _ => (),
                }
                unified_actions.insert(action);
            }
        }
        if unified_actions.len() == 1 {
            invariant_states.insert(state);
        }
    }
    let inline_states: HashSet<_> = preds
        .iter()
        .filter(|(_, a)| a.len() == 1)
        .map(|(&s, _)| s)
        .collect();

    debug!("{} invariant states", invariant_states.len());
    debug!("{} inline states", inline_states.len());

    // Generate some code.
    for (&nt, &state) in &ctx.lr_table.root_states {
        debug!("Root {} {} {:#?}", nt, state, state.items);

        // Group the actions.
        let mut ambiguous = BTreeSet::new();
        let mut actions = BTreeMap::<Action, BTreeSet<Term>>::new();
        for (&sym, acs) in &ctx.lr_table.actions[&state] {
            match sym {
                Symbol::Term(term) => {
                    if acs.len() > 1 {
                        ambiguous.insert(term);
                    } else {
                        for &action in acs {
                            actions.entry(action).or_default().insert(term);
                        }
                    }
                }
                _ => (),
            }
        }

        trace!("Ambiguous: {:?}", ambiguous);
        trace!("Actions:");
        for (action, terms) in &actions {
            trace!("  {} upon {:?}", action, terms);
        }

        // Generate some code.
        write!(into, "\n")?;
        write!(
            into,
            "fn action_{}(ctx: &mut Context, p: &mut impl AbstractParser) -> ReportedResult<()> {{\n",
            state
        )?;
        write!(into, "let t = p.peek(0);\n")?;
        write!(into, "match t.0 {{\n")?;

        // Add handling of recognized tokens.
        for (&action, terms) in &actions {
            write!(
                into,
                "{} => {{\n",
                terms
                    .iter()
                    .cloned()
                    .flat_map(match_pattern_for_terminal)
                    .format("|\n")
            )?;
            match action {
                Action::Shift(s) => write!(into, "// shift {}\n", s)?,
                Action::Reduce(p) => write!(into, "// reduce {}\n", p)?,
            }
            write!(into, "p.add_diag(DiagBuilder2::bug(")?;
            write!(into, "format!(\"not yet supported: `{{}}`\", t.0)")?;
            write!(into, ").span(t.1));\n")?;
            write!(into, "return Err(());\n")?;
            write!(into, "}}\n")?;
        }

        // Add handling of ambiguous code.
        if !ambiguous.is_empty() {
            write!(
                into,
                "{} => {{\n",
                ambiguous
                    .iter()
                    .cloned()
                    .flat_map(match_pattern_for_terminal)
                    .format("|\n")
            )?;
            write!(into, "p.add_diag(DiagBuilder2::bug(")?;
            write!(
                into,
                "format!(\"ambiguous: `{{}}` cannot be handled by the parser here\", t.0)"
            )?;
            write!(into, ").span(t.1));\n")?;
            write!(into, "return Err(());\n")?;
            write!(into, "}}\n")?;
        }

        // Add syntax error handling.
        write!(into, "_ => {{\n")?;
        write!(into, "p.add_diag(DiagBuilder2::error(")?;
        write!(
            into,
            "format!(\"syntax error: `{{}}` not possible here\", t.0)"
        )?;
        write!(into, ").span(t.1)\n")?;
        let expected: BTreeSet<_> = state
            .items
            .iter()
            .flat_map(|i| i.prod.nt.get_str())
            .map(|s| s.to_title_case().to_lowercase())
            .collect();
        for e in expected {
            write!(into, ".add_note(\"expected {}\")\n", e)?;
        }
        write!(into, ");\n")?;
        write!(into, "return Err(());\n")?;
        write!(into, "}}\n")?;

        write!(into, "}}\n")?;
        write!(into, "}}\n")?;
    }

    // let root = ctx.root_nonterms.clone();
    // let mut cg = Codegen::new(ctx);
    // for nt in root {
    //     debug!("Triggering root {}", nt);
    //     cg.schedule(nt);
    // }
    // cg.generate();
    Ok(())
}

fn match_pattern_for_terminal(term: Term) -> Option<String> {
    // TODO: This is super-hacky. Derive this from the grammar later on.
    Some(match term.as_str() {
        "$" => "Token::Eof".to_string(),
        "';'" => "Token::Semicolon".to_string(),
        "'('" => "Token::OpenDelim(DelimToken::Paren)".to_string(),
        "'['" => "Token::OpenDelim(DelimToken::Brack)".to_string(),
        "'{'" => "Token::OpenDelim(DelimToken::Brace)".to_string(),
        "')'" => "Token::CloseDelim(DelimToken::Paren)".to_string(),
        "']'" => "Token::CloseDelim(DelimToken::Brack)".to_string(),
        "'}'" => "Token::CloseDelim(DelimToken::Brace)".to_string(),
        "'IDENT'" => "Token::Ident(_)".to_string(),
        "'ATTR'" => return None,
        s if s.starts_with("'$") => return None,
        s if s.starts_with("'") => format!("Token::Keyword(Kw::{})", term.as_str().to_camel_case()),
        _ => return None,
    })
}

// struct Codegen<'a, 'b> {
//     ctx: &'b mut Context<'a>,
//     nonterm_seen: HashSet<Nonterm<'a>>,
//     nonterm_todo: VecDeque<Nonterm<'a>>,
//     nonterm_code: BTreeMap<Nonterm<'a>, Code<'a>>,
// }

// impl<'a, 'b> Codegen<'a, 'b> {
//     pub fn new(ctx: &'b mut Context<'a>) -> Self {
//         Codegen {
//             ctx,
//             nonterm_seen: Default::default(),
//             nonterm_todo: Default::default(),
//             nonterm_code: Default::default(),
//         }
//     }

//     pub fn schedule(&mut self, nt: Nonterm<'a>) {
//         if self.nonterm_seen.insert(nt) {
//             self.nonterm_todo.push_back(nt);
//         }
//     }

//     pub fn generate(&mut self) {
//         while let Some(nt) = self.nonterm_todo.pop_front() {
//             let code = self.emit_callable(nt);
//             self.nonterm_code.insert(nt, code);
//         }
//     }

//     fn emit_callable(&mut self, nt: Nonterm<'a>) -> Code<'a> {
//         // trace!("Generating {}", nt);

//         // Analyze the discriminant.
//         let mut prods = BTreeMap::<&'a Production<'a>, BTreeSet<Term<'a>>>::new();
//         let mut ambig = BTreeMap::<&'a BTreeSet<&'a Production<'a>>, BTreeSet<Term<'a>>>::new();
//         let mut epsilon_terms = BTreeSet::<Term<'a>>::new();
//         // let mut has_epsilon = false;
//         for (&t, ps) in &self.ctx.ll_table[&nt] {
//             for p in ps {
//                 prods.entry(p).or_default().insert(t);
//                 if p.is_epsilon {
//                     epsilon_terms.insert(t);
//                 }
//             }
//             if ps.len() > 1 {
//                 ambig.entry(ps).or_default().insert(t);
//             }
//         }

//         // Generate trivial code if possible.
//         if prods.len() == 1 {
//             return self.emit_production(prods.keys().nth(0).unwrap());
//         }

//         // Handle ambiguous code.
//         if !ambig.is_empty() {
//             trace!("Ambiguity in {}:", nt);
//             for (&ts, ps) in &ambig {
//                 for t in ts {
//                     trace!("  {}", t);
//                 }
//                 trace!("  = {:?}", ps);
//             }
//             return Code::Error;
//         }

//         // Generate a simple match code.
//         let mut cases = vec![];
//         for (p, ts) in prods {
//             let code = self.emit_production(p);
//             cases.push((ts, Box::new(code)));
//         }
//         let code = Code::Match {
//             cases,
//             expect: vec![nt].into_iter().collect(),
//         };
//         code
//     }

//     fn emit_production(&mut self, p: &'a Production<'a>) -> Code<'a> {
//         let mut actions = vec![];
//         for &sym in &p.syms {
//             actions.push(self.emit_symbol(sym));
//         }
//         Code::Actions(actions)
//     }

//     fn emit_symbol(&mut self, sym: Symbol<'a>) -> Code<'a> {
//         match sym {
//             Symbol::This => unreachable!(),
//             Symbol::Term(t) => Code::Require(t),
//             Symbol::Nonterm(nt) => {
//                 self.schedule(nt);
//                 Code::CallNonterm(nt)
//             }
//         }
//     }
// }

// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
// enum Code<'a> {
//     Error,
//     Require(Term<'a>),
//     CallNonterm(Nonterm<'a>),
//     Actions(Vec<Code<'a>>),
//     Match {
//         cases: Vec<(BTreeSet<Term<'a>>, Box<Code<'a>>)>,
//         expect: BTreeSet<Nonterm<'a>>,
//     },
// }
