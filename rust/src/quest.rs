//! Система квестов. Чистый Rust.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuestState {
    Active,
    Completed,
}

#[derive(Clone, Debug)]
pub struct Quest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub state: QuestState,
}

#[derive(Default)]
pub struct QuestLog {
    pub quests: Vec<Quest>,
}

impl QuestLog {
    pub fn has(&self, id: &str) -> bool {
        self.quests.iter().any(|q| q.id == id)
    }

    pub fn state_of(&self, id: &str) -> Option<QuestState> {
        self.quests.iter().find(|q| q.id == id).map(|q| q.state)
    }

    pub fn add(&mut self, id: &str, title: &str, description: &str) {
        if !self.has(id) {
            self.quests.push(Quest {
                id: id.to_string(),
                title: title.to_string(),
                description: description.to_string(),
                state: QuestState::Active,
            });
        }
    }

    pub fn complete(&mut self, id: &str) {
        if let Some(q) = self.quests.iter_mut().find(|q| q.id == id) {
            q.state = QuestState::Completed;
        }
    }

    pub fn is_active(&self, id: &str) -> bool {
        self.state_of(id) == Some(QuestState::Active)
    }

    pub fn is_completed(&self, id: &str) -> bool {
        self.state_of(id) == Some(QuestState::Completed)
    }

    pub fn is_empty(&self) -> bool {
        self.quests.is_empty()
    }
}
