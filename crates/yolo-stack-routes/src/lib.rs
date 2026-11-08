//! Shared route table used by every showcase app (TanStack, Dioxus, Leptos) so the
//! benchmark harness hits identical pages on each stack.
//! The TypeScript twin lives in packages/yolo-stack-routes/src/index.ts; keep both in sync.

/// One benchmarked route. `rows` is the number of list rows the page renders server-side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    pub id: &'static str,
    pub path: &'static str,
    pub rows: u32,
    pub interactive: bool,
}

/// The canonical route set. Order matters: the harness reports in this order.
pub const ROUTES: &[Route] = &[
    Route { id: "home", path: "/", rows: 0, interactive: false },
    Route { id: "links-100", path: "/links", rows: 100, interactive: false },
    Route { id: "table-1k", path: "/table", rows: 1000, interactive: true },
    Route { id: "island-counter", path: "/island", rows: 0, interactive: true },
    Route { id: "api-json", path: "/api/health", rows: 0, interactive: false },
];

/// Look a route up by id.
pub fn route(id: &str) -> Option<&'static Route> {
    ROUTES.iter().find(|r| r.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<&str> = ROUTES.iter().map(|r| r.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ROUTES.len());
    }

    #[test]
    fn lookup() {
        assert_eq!(route("table-1k").map(|r| r.rows), Some(1000));
        assert!(route("missing").is_none());
    }
}
