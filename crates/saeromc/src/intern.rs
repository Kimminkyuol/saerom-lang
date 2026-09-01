use std::collections::HashMap;
use std::rc::Rc;

pub type Symbol = u32;

#[derive(Default)]
pub struct Interner {
    seen: HashMap<Rc<str>, Symbol>,
    names: Vec<Rc<str>>,
}

impl Interner {
    pub fn intern(&mut self, text: &str) -> Symbol {
        if let Some(&found) = self.seen.get(text) {
            return found;
        }
        let shared: Rc<str> = Rc::from(text);
        let id = self.names.len() as Symbol;
        self.names.push(shared.clone());
        self.seen.insert(shared, id);
        id
    }

    pub fn name(&self, symbol: Symbol) -> &str {
        &self.names[symbol as usize]
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}
