fn marker(kind: u8, index: usize) -> NavMarker {
    match kind {
        0 => NavMarker::ViewOpen(index),
        1 => NavMarker::ViewClose,
        2 => NavMarker::TabOpen(index),
        _ => NavMarker::TabClose,
    }
}

fn assert_assignment(events: &[NavMarker], view_count: usize) {
    let result = simulate_tabview_navs(events, view_count);
    let Some(assigned) = result else {
        return;
    };
    assert_eq!(assigned.len(), view_count);
    let opened = events.iter().filter_map(|event| match event {
        NavMarker::TabOpen(index) => Some(*index),
        _ => None,
    });
    let known: Vec<usize> = opened.collect();
    for view in &assigned {
        for tab in view {
            assert!(
                known.contains(tab),
                "assigned tab {tab} is not an opened tab"
            );
        }
    }
}

#[test]
fn tab_nav_flat_match_permutes_the_known_nest() {
    let events = vec![
        NavMarker::ViewOpen(0),
        NavMarker::TabOpen(0),
        NavMarker::ViewOpen(1),
        NavMarker::TabOpen(1),
        NavMarker::TabClose,
        NavMarker::ViewClose,
        NavMarker::TabClose,
        NavMarker::TabOpen(2),
        NavMarker::TabClose,
        NavMarker::ViewClose,
    ];
    assert_eq!(
        simulate_tabview_navs(&events, 2),
        Some(vec![vec![0], vec![1, 2]])
    );
}

#[test]
fn tab_nav_duplicate_view_assignment_is_rejected() {
    let events = vec![
        NavMarker::ViewOpen(0),
        NavMarker::TabOpen(0),
        NavMarker::TabClose,
        NavMarker::ViewClose,
        NavMarker::ViewOpen(0),
        NavMarker::TabOpen(1),
        NavMarker::TabClose,
        NavMarker::ViewClose,
    ];
    assert_eq!(simulate_tabview_navs(&events, 1), None);
}

#[test]
fn tab_nav_generated_sequences_assign_each_view_or_none() {
    let view_count = 2usize;
    for length in 0..=6 {
        let total = 4u32.pow(length as u32);
        let step = (total / 48).max(1);
        let mut code = 0u32;
        while code < total {
            let mut events = Vec::with_capacity(length);
            let mut rest = code;
            for _ in 0..length {
                let kind = (rest % 4) as u8;
                rest /= 4;
                let index = match kind {
                    0 => (rest % 2) as usize,
                    2 => (rest % 3) as usize,
                    _ => 0,
                };
                events.push(marker(kind, index));
            }
            assert_assignment(&events, view_count);
            code = code.saturating_add(step);
        }
    }
}
