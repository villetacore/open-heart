//! Инвентарь и предметы. Чистый Rust.

#[derive(Clone, Debug)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub description: String,
    pub qty: u32,
}

impl Item {
    pub fn new(id: &str, name: &str, description: &str, qty: u32) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            qty,
        }
    }
}

#[derive(Default)]
pub struct Inventory {
    pub items: Vec<Item>,
}

impl Inventory {
    /// Добавить предмет; если такой id уже есть — увеличить количество.
    pub fn add(&mut self, item: Item) {
        if let Some(existing) = self.items.iter_mut().find(|i| i.id == item.id) {
            existing.qty += item.qty;
        } else {
            self.items.push(item);
        }
    }

    pub fn has(&self, id: &str) -> bool {
        self.items.iter().any(|i| i.id == id && i.qty > 0)
    }

    /// Убрать одну единицу предмета; вернуть true, если получилось.
    pub fn remove_one(&mut self, id: &str) -> bool {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id && i.qty > 0) {
            item.qty -= 1;
            self.items.retain(|i| i.qty > 0);
            true
        } else {
            false
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
