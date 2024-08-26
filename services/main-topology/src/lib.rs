// Topology definition with actor groups and connections between them.
pub fn topology() -> elfo::Topology {
    let topology = elfo::Topology::empty();

    // Set up logging (based on the `tracing` crate).
    // `elfo` provides a logger actor group to support runtime control.
    //
    // Also, you can filter logs by passing `RUST_LOG`:
    // * `RUST_LOG=elfo`
    // * `RUST_LOG=info,[{actor_group=aggregators}]`
    //
    // However, it's more useful to control logging in the config file.
    let logger = elfo::batteries::logger::init();
    // Setup up telemetry (based on the `metrics` crate).
    // let telemeter = elfo::batteries::telemeter::init();

    // Define actor groups.
    let fs_watcher = topology.local("fs-watcher");
    // let producers = topology.local("producers");
    // let aggregators = topology.local("aggregators");
    // let reporters = topology.local("reporters");
    let loggers = topology.local("system.loggers");
    // let telemeters = topology.local("system.telemeters");
    // let dumpers = topology.local("system.dumpers");
    // let pingers = topology.local("system.pingers");
    let configurers = topology.local("system.configurers").entrypoint();

    //WARN:
    // Define links between actor groups.
    // Producers send raw data to aggregators.
    // producers.route_all_to(&aggregators);
    // Reporters ask aggregators for a summary.
    // reporters.route_all_to(&aggregators);

    // Mount specific implementations.
    fs_watcher.mount(watcher::new());

    //WARN:
    // producers.mount(producer::new());
    // aggregators.mount(aggregator::new());
    // reporters.mount(reporter::new());

    loggers.mount(logger);
    // telemeters.mount(telemeter);
    // dumpers.mount(elfo::batteries::dumper::new());
    // pingers.mount(elfo::batteries::pinger::new(&topology));

    // Actors can use `topology` as an extended service locator.
    // Usually it should be used for utilities only.
    // TODO: вынести в отедльную папку с конфигами
    let config_path = "/home/data/Work/rust/FileOrganizer/config.toml";
    configurers.mount(elfo::batteries::configurer::from_path(
        &topology,
        config_path,
    ));

    topology
}
