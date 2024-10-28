use fs::resolve_path;

// Topology definition with actor groups and connections between them.
pub fn topology() -> elfo::Topology {
    let topology = elfo::Topology::empty();

    // However, it's more useful to control logging in the config file.
    let logger = elfo::batteries::logger::init();

    // Define actor groups.
    let fs_watcher = topology.local("fs-watcher");
    let executors = topology.local("executors");
    let loggers = topology.local("system.loggers");
    let configurers = topology.local("system.configurers").entrypoint();

    fs_watcher.route_all_to(&executors);

    // Mount specific implementations.
    fs_watcher.mount(watcher::new());

    executors.mount(executor::new());

    loggers.mount(logger);

    let config_path = resolve_path("~/.config/TriggerFS/config.toml");

    configurers.mount(elfo::batteries::configurer::from_path(
        &topology,
        config_path,
    ));

    topology
}
