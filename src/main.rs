use stanchion::{
    flow::{
        MaxFlowSolver, MinCostFlowSolver,
        dinic::Dinic,
        min_cost::{SuccessiveShortest, CycleCanceling},
        scaling::CapacityScaling,
    },
    graph::{Capacity, Cost, GraphBuilder},
    opt::OptimizationEngine,
    queue::{ExternalArrivals, JacksonNetwork, RoutingMatrix, ServiceRates},
    scheduling::{MaxWeightScheduler, QueueState},
    sim::{ExponentialService, PoissonArrival, SimConfig, SimulationEngine},
};

fn main() {
    // Airport security stanchion network:
    //
    //   entry(0) -> lane_a(1) -> security(3)
    //            -> lane_b(2) -> security(3)
    //
    // lane_a: capacity 3, cost 1 (fast lane)
    // lane_b: capacity 2, cost 2 (standard lane)
    let (b, entry)    = GraphBuilder::new().add_stanchion("entry");
    let (b, lane_a)   = b.add_stanchion("lane_a");
    let (b, lane_b)   = b.add_stanchion("lane_b");
    let (b, security) = b.add_stanchion("security");
    let graph = b
        .source(entry)
        .sink(security)
        .connect(entry,  lane_a,   Capacity(3.0), Cost(1.0))
        .connect(entry,  lane_b,   Capacity(2.0), Cost(2.0))
        .connect(lane_a, security, Capacity(3.0), Cost(1.0))
        .connect(lane_b, security, Capacity(2.0), Cost(2.0))
        .build()
        .expect("valid graph");

    // --- Max-flow (Dinic) ---
    let flow = Dinic.max_flow(&graph, entry, security).expect("flow");
    println!("max flow (Dinic):        {:.1}", flow.max_flow);

    // --- Max-flow (capacity scaling) ---
    let flow_cs = CapacityScaling.max_flow(&graph, entry, security).expect("flow");
    println!("max flow (cap-scaling):  {:.1}", flow_cs.max_flow);

    // --- Min-cost flow (SSP) ---
    let mcf = SuccessiveShortest
        .min_cost_flow(&graph, entry, security, None)
        .expect("mcf");
    println!("min-cost (SSP):          {:.1}  cost: {:.1}", mcf.max_flow, mcf.min_cost);

    // --- Min-cost flow (cycle canceling) ---
    let mcf_cc = CycleCanceling
        .min_cost_flow(&graph, entry, security, None)
        .expect("mcf-cc");
    println!("min-cost (cycle-cancel): {:.1}  cost: {:.1}", mcf_cc.max_flow, mcf_cc.min_cost);

    // --- Max-weight scheduler demo ---
    let mut qs = QueueState::new(graph.node_count());
    qs.queue_len[1] = 3; // lane_a has 3 waiting
    qs.queue_len[2] = 7; // lane_b has 7 waiting
    let sched = MaxWeightScheduler::new(graph.node_count());
    if let Some(node) = sched.next_to_serve(&qs) {
        println!("MW scheduler: serve node {} (longest queue)", node.0);
    }

    // --- Jackson queueing network ---
    // External arrivals only at entry (2 people/s).
    // entry routes 60% to lane_a, 40% to lane_b; lanes route 100% to security.
    let net = JacksonNetwork {
        arrivals: ExternalArrivals(vec![2.0, 0.0, 0.0, 0.0]),
        service:  ServiceRates(vec![10.0, 3.0, 2.0, 5.0]),
        routing:  RoutingMatrix(vec![
            vec![0.0, 0.6, 0.4, 0.0],
            vec![0.0, 0.0, 0.0, 1.0],
            vec![0.0, 0.0, 0.0, 1.0],
            vec![0.0, 0.0, 0.0, 0.0],
        ]),
    };
    match net.steady_state() {
        Ok(ss) => {
            println!("\nJackson steady state:");
            let labels = ["entry", "lane_a", "lane_b", "security"];
            for (i, label) in labels.iter().enumerate() {
                println!(
                    "  {:<10} lambda={:.2}  rho={:.2}  L={:.2}",
                    label, ss.throughput[i], ss.utilization[i], ss.mean_queue_len[i]
                );
            }
            println!("  stable: {}", ss.stable);
        }
        Err(e) => eprintln!("jackson error: {e}"),
    }

    // --- Event-driven simulation ---
    let arrival = PoissonArrival::new(2.0, 42);
    let service  = ExponentialService::uniform(3.0, graph.node_count(), 7);
    let mut engine = SimulationEngine::new(arrival, service);
    let config = SimConfig { max_time: 1000.0, max_events: None, seed: 99 };
    match engine.run(&graph, &config) {
        Ok(stats) => {
            println!("\nSimulation (T=1000):");
            println!("  arrivals:   {}", stats.total_arrivals);
            println!("  departures: {}", stats.total_departures);
            println!("  throughput: {:.3}", stats.throughput);
        }
        Err(e) => eprintln!("sim error: {e}"),
    }

    // --- Optimization recommendations ---
    let opt = OptimizationEngine::new(Dinic);
    match opt.recommend(&graph, entry, security) {
        Ok(decisions) => {
            println!("\nOptimizer recommendations ({} total):", decisions.len());
            for d in decisions.iter().take(3) {
                println!("  {:?}", d);
            }
        }
        Err(e) => eprintln!("optimizer error: {e}"),
    }
}
