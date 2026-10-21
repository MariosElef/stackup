use crate::ui::{features::create::wizard::Wizard, types::Tab};

impl Wizard {
    pub fn scroll_by(&mut self, delta: isize) {
        let max = self.max_scroll.get() as isize;
        self.scroll = (self.scroll as isize + delta).clamp(0, max) as u16;
    }

    pub fn switch_tab(&mut self) {
        self.tab = if self.tab == Tab::Files {
            Tab::Compose
        } else {
            Tab::Files
        };
        self.scroll = 0;
    }
}
