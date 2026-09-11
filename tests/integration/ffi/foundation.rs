use meshchat_core::*;

fn core() -> Core {
    Core::new(
        Limits {
            max_links: 1,
            max_value_bytes: 64,
        },
        42,
        100,
    )
    .unwrap()
}
fn capacities() -> Capacities {
    Capacities {
        write_bytes: 20,
        notify_bytes: 12,
        receive_bytes: 30,
    }
}
fn connect(core: &Core) -> LinkHandle {
    let effect = core.connected(capacities()).unwrap();
    match effect.ui_events.first().unwrap() {
        UiEvent::LinkConnected { link } => link.clone(),
        _ => panic!("expected connection"),
    }
}
fn trace() -> Vec<Effects> {
    let core = core();
    let link = connect(&core);
    vec![
        core.handle_event(DriverEvent::TimeAdvanced { monotonic_ms: 101 })
            .unwrap(),
        core.prepare_send(link.clone(), SendPath::Write, vec![0, 127, 255])
            .unwrap(),
        core.handle_event(DriverEvent::InboundBytes {
            link: link.clone(),
            bytes: vec![0, 127, 255],
        })
        .unwrap(),
        core.handle_event(DriverEvent::PowerChanged {
            state: PowerState::LowPower,
        })
        .unwrap(),
        core.handle_event(DriverEvent::Disconnected { link })
            .unwrap(),
    ]
}
#[test]
fn deterministic_trace_and_exact_binary_round_trip() {
    let result = trace();
    assert_eq!(result, trace());
    assert_eq!(result[1].sends[0].bytes, [0, 127, 255]);
    assert_eq!(
        result[2].ui_events,
        [UiEvent::InboundObserved {
            link: LinkHandle {
                instance_nonce: 42,
                generation: 1
            },
            byte_count: 3,
        }]
    );
    for effects in result {
        assert!(effects.sends.len() + effects.ui_events.len() <= 1);
    }
}
#[test]
fn invalid_input_is_transactional_and_disconnect_releases_capacity() {
    let core = core();
    let link = connect(&core);
    assert_eq!(core.connected(capacities()), Err(CoreError::LinkLimit));
    for bytes in [vec![], vec![1; 31]] {
        assert_eq!(
            core.handle_event(DriverEvent::InboundBytes {
                link: link.clone(),
                bytes
            }),
            Err(CoreError::InvalidValue)
        );
    }
    assert_eq!(
        core.handle_event(DriverEvent::CapacitiesChanged {
            link: link.clone(),
            capacities: Capacities {
                write_bytes: 0,
                notify_bytes: 0,
                receive_bytes: 1
            },
        }),
        Err(CoreError::InvalidCapacity)
    );
    assert!(
        core.prepare_send(link.clone(), SendPath::Write, vec![1; 20])
            .is_ok()
    );
    assert_eq!(
        core.prepare_send(link.clone(), SendPath::Notify, vec![1; 13]),
        Err(CoreError::InvalidValue)
    );
    core.handle_event(DriverEvent::Disconnected { link: link.clone() })
        .unwrap();
    let replacement = connect(&core);
    assert_ne!(link, replacement);
    assert_eq!(
        core.handle_event(DriverEvent::Disconnected { link: link.clone() }),
        Err(CoreError::UnknownLink)
    );
    assert_eq!(
        core.prepare_send(link, SendPath::Write, vec![1]),
        Err(CoreError::UnknownLink)
    );
    assert!(
        core.prepare_send(replacement, SendPath::Write, vec![1])
            .is_ok()
    );
}
#[test]
fn clock_and_cross_instance_handles_are_checked() {
    let core = core();
    let mut link = connect(&core);
    link.instance_nonce = 99;
    assert_eq!(
        core.prepare_send(link, SendPath::Write, vec![1]),
        Err(CoreError::UnknownLink)
    );
    assert_eq!(
        core.handle_event(DriverEvent::TimeAdvanced { monotonic_ms: 99 }),
        Err(CoreError::TimeRegression)
    );
    assert!(
        core.handle_event(DriverEvent::TimeAdvanced { monotonic_ms: 100 })
            .is_ok()
    );
    assert!(
        core.handle_event(DriverEvent::TimeAdvanced {
            monotonic_ms: u64::MAX
        })
        .is_ok()
    );
    assert_eq!(
        core.handle_event(DriverEvent::TimeAdvanced { monotonic_ms: 0 }),
        Err(CoreError::TimeRegression)
    );
}
#[test]
fn directional_changes_apply_immediately() {
    let core = core();
    let link = connect(&core);
    core.handle_event(DriverEvent::CapacitiesChanged {
        link: link.clone(),
        capacities: Capacities {
            write_bytes: 0,
            notify_bytes: 5,
            receive_bytes: 1,
        },
    })
    .unwrap();
    assert_eq!(
        core.prepare_send(link.clone(), SendPath::Write, vec![1]),
        Err(CoreError::InvalidValue)
    );
    assert!(
        core.prepare_send(link.clone(), SendPath::Notify, vec![1; 5])
            .is_ok()
    );
    assert_eq!(
        core.handle_event(DriverEvent::InboundBytes {
            link,
            bytes: vec![1, 2]
        }),
        Err(CoreError::InvalidValue)
    );
}
