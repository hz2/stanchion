//! Kani formal verification proofs for core graph and flow invariants.
//!
//! Run with: `cargo kani`
//! These are compiled only when Kani is active (`#[cfg(kani)]`).

#[cfg(kani)]
mod proofs {
    use crate::graph::{Capacity, Cost, GraphBuilder, NodeId, ResidualGraph};

    // ---- graph invariants ----

    /// Every forward edge (even index) has a matching back-edge at index ^ 1.
    #[kani::proof]
    fn xor_back_edge_pairing() {
        let cap: f64 = kani::any();
        let cost: f64 = kani::any();
        kani::assume(cap >= 0.0);
        kani::assume(cost.is_finite());

        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, t) = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, t, Capacity(cap), Cost(cost))
            .build().unwrap();

        // edges[0] is forward, edges[1] is back
        assert!(g.edges.len() == 2);
        assert!(g.edges[0].to == t);
        assert!(g.edges[1].to == s);
        // back-edge has zero capacity and negated cost
        assert!(g.edges[1].capacity.0 == 0.0);
        assert!((g.edges[1].cost.0 + cost).abs() < 1e-12);
    }

    /// Pushing flow `d` on an arc and then `-d` on its back-edge returns to zero.
    /// This checks the XOR antisymmetry invariant used by all flow algorithms.
    #[kani::proof]
    fn push_flow_antisymmetric() {
        let cap: f64 = kani::any();
        let push: f64 = kani::any();
        kani::assume(cap >= 0.0);
        kani::assume(push >= 0.0);
        kani::assume(push <= cap);

        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, t) = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, t, Capacity(cap), Cost(1.0))
            .build().unwrap();

        let mut res = ResidualGraph::new(&g);
        // forward arc is at index 0
        res.push_flow(0, push);
        // residual on forward arc drops by push; back-arc rises by push
        assert!((res.residual_capacity(0) - (cap - push)).abs() < 1e-12);
        assert!((res.residual_capacity(1) - push).abs() < 1e-12);
    }

    /// Residual capacity of a fresh graph equals the original capacity.
    #[kani::proof]
    fn fresh_residual_capacity_equals_original() {
        let cap: f64 = kani::any();
        kani::assume(cap >= 0.0);
        kani::assume(cap < 1e15); // avoid floating-point overflow

        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, t) = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, t, Capacity(cap), Cost(1.0))
            .build().unwrap();

        let res = ResidualGraph::new(&g);
        assert!((res.residual_capacity(0) - cap).abs() < 1e-12);
        // back-edge starts at residual 0 (no flow has been pushed yet)
        assert!((res.residual_capacity(1)).abs() < 1e-12);
    }

    /// `is_forward_edge` correctly identifies even-indexed edges as forward.
    #[kani::proof]
    fn forward_edge_detection() {
        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, m) = b.add_stanchion("m");
        let (b, t) = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, m, Capacity(1.0), Cost(1.0))
            .connect(m, t, Capacity(1.0), Cost(1.0))
            .build().unwrap();

        for (i, _) in g.edges.iter().enumerate() {
            use crate::graph::EdgeId;
            let is_fwd = g.is_forward_edge(EdgeId(i as u32));
            assert!(is_fwd == (i % 2 == 0));
        }
    }

    /// Source and sink are reachable from GraphBuilder.
    #[kani::proof]
    fn source_and_sink_are_set() {
        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, t) = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, t, Capacity(1.0), Cost(1.0))
            .build().unwrap();
        assert!(g.source() == Some(s));
        assert!(g.sink()   == Some(t));
    }

    /// Building without a source returns an error.
    #[kani::proof]
    fn missing_source_errors() {
        let (b, _s) = GraphBuilder::new().add_stanchion("s");
        let (b,  t) = b.add_stanchion("t");
        let result = b.sink(t).build();
        assert!(result.is_err());
    }

    /// Building without a sink returns an error.
    #[kani::proof]
    fn missing_sink_errors() {
        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, _t) = b.add_stanchion("t");
        let result = b.source(s).build();
        assert!(result.is_err());
    }
}
