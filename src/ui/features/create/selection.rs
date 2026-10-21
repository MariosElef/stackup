use clap::ValueEnum;

use crate::{
    models::{Backend, BeLib, FeLib, Frontend, Service, Tool},
    ui::utils::toggle,
};

#[derive(Clone)]
pub struct Selection {
    pub frontend: Frontend,
    pub backend: Backend,
    pub libs: Vec<FeLib>,
    pub be_libs: Vec<BeLib>,
    pub services: Vec<Service>,
    pub tools: Vec<Tool>,
}

impl Selection {
    pub fn recommended() -> Self {
        Self {
            frontend: Frontend::Nuxt,
            backend: Backend::Axum,
            libs: vec![FeLib::Pinia, FeLib::Colada],
            be_libs: vec![],
            services: vec![Service::Postgres, Service::Redis],
            tools: vec![Tool::Gitleaks, Tool::Lint, Tool::Ci, Tool::Git],
        }
    }

    /// These choices as the flags of `stackup new`, so they can be handed to it: the menu runs
    /// it on its in-app terminal. A list with nothing in it is the bare flag, which clap reads as
    /// an empty list (leaving it out would open the wizard again).
    pub fn cli_args(&self) -> Vec<String> {
        fn name<T: ValueEnum>(v: &T) -> String {
            v.to_possible_value()
                .map(|p| p.get_name().to_string())
                .unwrap_or_default()
        }
        fn list<T: ValueEnum>(flag: &str, items: &[T], out: &mut Vec<String>) {
            out.push(flag.to_string());
            if !items.is_empty() {
                out.push(items.iter().map(name).collect::<Vec<_>>().join(","));
            }
        }
        let mut out = vec![
            "--frontend".to_string(),
            name(&self.frontend),
            "--backend".to_string(),
            name(&self.backend),
        ];
        list("--libs", &self.libs, &mut out);
        list("--backend-libs", &self.be_libs, &mut out);
        list("--services", &self.services, &mut out);
        list("--tools", &self.tools, &mut out);
        out
    }

    /// Backend libraries belong to a backend; says why when the chosen one cannot use this one.
    pub fn toggle_be_lib(&mut self, lib: BeLib) -> Result<(), &'static str> {
        if !lib.works_with(self.backend) {
            return Err("utoipa needs the axum backend");
        }
        toggle(&mut self.be_libs, lib);
        Ok(())
    }

    /// Libraries belong to a frontend; says why when the chosen one cannot use this library.
    pub fn toggle_lib(&mut self, lib: FeLib) -> Result<(), &'static str> {
        if self.frontend == Frontend::None {
            return Err("choose a frontend first");
        }
        if !lib.works_with(self.frontend) {
            return Err("not available for this frontend");
        }
        toggle(&mut self.libs, lib);
        // Pinia Colada is built on Pinia: turning one on or the other off drags its partner.
        if lib == FeLib::Colada && self.libs.contains(&lib) && !self.libs.contains(&FeLib::Pinia) {
            self.libs.push(FeLib::Pinia);
        }
        if lib == FeLib::Pinia && !self.libs.contains(&lib) {
            self.libs.retain(|l| *l != FeLib::Colada);
        }
        Ok(())
    }
}
