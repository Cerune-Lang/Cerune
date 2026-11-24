use cerune_lang::runtime::trace::{
    BoundedEvents,
    file::{Config, Fields},
};
use wasmi::{Error, Module};

pub(super) struct Capture {
    config: Config,
    sites: Vec<Fields<String>>,
    events: BoundedEvents<usize>,
}

impl Capture {
    pub fn open(module: &Module, config: Option<Config>) -> Result<Option<Self>, String> {
        let instrumented = module
            .imports()
            .any(|i| i.module() == "cerune" && i.name() == "trace_event_v1");
        let mut sections = module
            .custom_sections()
            .filter(|s| s.name() == "cerune.trace.v1");
        let section = sections.next();
        if instrumented != section.is_some() || sections.next().is_some() {
            return Err("trace_event_v1 requires one cerune.trace.v1 site table".into());
        }
        let Some(config) = config else {
            return Ok(None);
        };
        let section =
            section.ok_or("--trace requires a module built with emit-wat --with-trace")?;
        let table = std::str::from_utf8(section.data()).map_err(|_| "invalid trace site table")?;
        let sites = table
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()
            .map_err(|_| "invalid trace site record")?;
        let events = BoundedEvents::new(config.events);
        Ok(Some(Self {
            config,
            sites,
            events,
        }))
    }
    pub fn record(&mut self, id: i32) -> Result<(), Error> {
        let id = id as u32 as usize;
        if id >= self.sites.len() {
            return Err(Error::new("unknown trace site"));
        }
        self.events.record(id);
        Ok(())
    }
    pub fn save(&self) -> Result<(), String> {
        self.config
            .save_rows(self.events.events().len(), self.events.incomplete(), |i| {
                &self.sites[self.events.events()[i]]
            })
    }
}
