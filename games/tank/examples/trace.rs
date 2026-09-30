//! Debug trace: positions/hp every N ticks for a query.
fn main() {
    let q = std::env::args().nth(1).unwrap_or_default();
    let every: u32 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let spec = tank::MatchSpec::from_query(&q).unwrap();
    let (mut m, [mut a, mut b]) = spec.start();
    let mut shots = [0u32; 2];
    let mut hits = [0u32; 2];
    loop {
        let o = m.step_policies(&mut [a.as_mut(), b.as_mut()]);
        for e in m.events() {
            match e {
                engine::Event::Fired { tank } => {
                    shots[*tank] += 1;
                    if std::env::var("FIRES").is_ok() {
                        println!("  fire t={} tank={}", m.tick(), tank);
                    }
                }
                engine::Event::Hit { owner, .. } => hits[*owner] += 1,
                _ => {}
            }
        }
        if m.tick() % every == 0 || o.is_some() {
            let t = m.tanks();
            println!(
                "t={:4} B({:5.0},{:5.0}) hp{:4} | O({:5.0},{:5.0}) hp{:4} | shots {:?} hits {:?}",
                m.tick(),
                t[0].pos.x,
                t[0].pos.y,
                t[0].hp,
                t[1].pos.x,
                t[1].pos.y,
                t[1].hp,
                shots,
                hits
            );
        }
        if let Some(o) = o {
            println!("{o:?}");
            break;
        }
    }
}
