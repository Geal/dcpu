use std::collections::HashMap;
use std::iter;

use assembler::types::*;

error_chain!{
    errors {
        UnknownLabel(l: String) {
            description("unknown label")
            display("unknown label: \"{}\"", l)
        }
        UnknownLocalLabel(l: String) {
            description("unknown local label")
            display("unknown local label: \"{}\"", l)
        }
        DuplicatedLabel(l: String) {
            description("duplicated label")
            display("duplicated label: \"{}\"", l)
        }
        DuplicatedLocalLabel(l: String) {
            description("duplicated local label")
            display("duplicated local label: \"{}\"", l)
        }
        LocalBeforeGlobal(l: String) {
            description("local label before a global")
            display("local label before a global: \"{}\"", l)
        }
    }
}

pub fn link(ast: &[ParsedItem]) -> Result<(Vec<u16>, Globals)> {

    let mut bin = Vec::new();
    let mut labels = try!(extract_labels(ast));
    let mut changed = true;

    while changed {
        bin.clear();
        changed = false;
        let mut index = 0u16;
        let mut last_global = None;
        for item in ast {
            match *item {
                ParsedItem::Directive(Directive::Lcomm(ref symbol, size)) => {
                    let label = labels.get_mut(symbol).unwrap();
                    if label.addr != index {
                        label.addr = index;
                        changed = true;
                    }
                    last_global = Some(symbol.clone());

                    bin.extend(iter::repeat(0).take(size as usize));
                    index += size;
                }
                ParsedItem::Directive(ref d) =>
                    index += try!(d.append_to(&mut bin, &labels, &last_global)),
                ParsedItem::LabelDecl(ref s) => {
                    let label = labels.get_mut(s).unwrap();
                    if label.addr != index {
                        label.addr = index;
                        changed = true;
                    }
                    last_global = Some(s.clone());
                }
                ParsedItem::LocalLabelDecl(ref s) => {
                    let addr = labels.get_mut(last_global.as_ref().unwrap())
                                     .unwrap()
                                     .locals
                                     .get_mut(s)
                                     .unwrap();
                    if *addr != index {
                        changed = true;
                        *addr = index;
                    }
                }
                ParsedItem::Instruction(ref i) => {
                    let solved = try!(i.solve(&labels, &last_global));
                    bin.extend_from_slice(&[0xbeaf; 3]);
                    index += solved.encode(&mut bin[index as usize..]);
                    bin.truncate(index as usize);
                }
                _ => (),
            }
        }
    }

    Ok((bin, labels))
}

fn extract_labels(ast: &[ParsedItem]) -> Result<Globals> {
    let mut prev_label = None;
    let mut labels = HashMap::new();

    for item in ast.iter() {
        match *item {
            ParsedItem::LabelDecl(ref s) | ParsedItem::Directive(Directive::Lcomm(ref s, _)) => {
                prev_label = Some(s.clone());
                if labels.contains_key(s) {
                    try!(Err(ErrorKind::DuplicatedLabel(s.clone())));
                } else {
                    labels.insert(s.clone(), LabelInfos::default());
                }
            }
            ParsedItem::LocalLabelDecl(ref s) => {
                if prev_label.is_none() {
                    try!(Err(ErrorKind::LocalBeforeGlobal(s.clone())));
                }
                let locals = &mut labels.get_mut(prev_label.as_ref().unwrap())
                                        .unwrap()
                                        .locals;
                if locals.contains_key(s) {
                    try!(Err(ErrorKind::DuplicatedLocalLabel(s.clone())));
                } else {
                    locals.insert(s.clone(), 0);
                }
            }
            _ => (),
        }
    }

    Ok(labels)
}
