//! Validates that the event-driven simulation converges to Jackson steady state.

use stanchion::{
    graph::{Capacity, Cost, GraphBuilder},
    queue::{ExternalArrivals, JacksonNetwork, RoutingMatrix, ServiceRates},
    sim::{ExponentialService, PoissonArrival, SimConfig, SimulationEngine},
};

#[test]
fn single_mm1_throughput_converges() {
    // M/M/1: lambda=2, mu=5, rho=0.4 => L = rho/(1-rho) = 0.667
    let (b, s) = GraphBuilder::new().add_stanchion("server");
    let (b, t) = b.add_stanchion("exit");
    let g = b.source(s).sink(t)
        .connect(s, t, Capacity(10.0), Cost(1.0))
        .build().unwrap();

    let arrival = PoissonArrival::new(2.0, 1);
    let service  = ExponentialService::uniform(5.0, g.node_count(), 2);
    let mut engine = SimulationEngine::new(arrival, service);
    let cfg = SimConfig { max_time: 5000.0, max_events: None, seed: 3 };
    let stats = engine.run(&g, &cfg).unwrap();

    // throughput should be ~2.0 (lambda), within 5%
    let expected = 2.0f64;
    let actual   = stats.throughput;
    assert!(
        (actual - expected).abs() / expected < 0.05,
        "throughput {actual:.4} not within 5% of {expected}"
    );
}

#[test]
fn jackson_two_stage_throughput_converges() {
    // Two-stage: entry -> lane -> exit
    // lambda=2, mu_lane=4, mu_exit=6
    // Jackson: lambda_entry=2, lambda_lane=2, lambda_exit=2
    let (b, entry) = GraphBuilder::new().add_stanchion("entry");
    let (b, lane)  = b.add_stanchion("lane");
    let (b, exit)  = b.add_stanchion("exit");
    let g = b.source(entry).sink(exit)
        .connect(entry, lane, Capacity(10.0), Cost(1.0))
        .connect(lane,  exit, Capacity(10.0), Cost(1.0))
        .build().unwrap();

    // Validate Jackson prediction first
    let net = JacksonNetwork {
        arrivals: ExternalArrivals(vec![2.0, 0.0, 0.0]),
        service:  ServiceRates(vec![10.0, 4.0, 6.0]),
        routing:  RoutingMatrix(vec![
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
            vec![0.0, 0.0, 0.0],
        ]),
    };
    let ss = net.steady_state().unwrap();
    assert!(ss.stable);

    // Simulate and compare
    let arrival = PoissonArrival::new(2.0, 10);
    let service  = ExponentialService::new(vec![10.0, 4.0, 6.0], 20);
    let mut engine = SimulationEngine::new(arrival, service);
    let cfg = SimConfig { max_time: 10_000.0, max_events: None, seed: 77 };
    let stats = engine.run(&g, &cfg).unwrap();

    // throughput should converge to ~2.0
    assert!(
        (stats.throughput - 2.0).abs() / 2.0 < 0.07,
        "throughput {:.4} not within 7% of 2.0", stats.throughput
    );
}
