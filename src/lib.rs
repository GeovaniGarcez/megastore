//! Grafo heterogeneo nao direcionado, com BFS limitada e ranking por evidencias.
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Vertex {
    Customer(u32),
    Product(u32),
    Category(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Relation {
    Purchase,
    Interest,
    BelongsTo,
    Similarity,
}

#[derive(Debug, Clone)]
pub struct Product {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub to: Vertex,
    pub relation: Relation,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError(pub String);
impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for StoreError {}
type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Clone)]
pub struct Recommendation {
    pub product_id: u32,
    pub score: f64,
    pub distance: usize,
}

#[derive(Default)]
pub struct Store {
    products: HashMap<u32, Product>,
    adjacency: HashMap<Vertex, Vec<Edge>>,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_product(&mut self, id: u32, name: &str) -> Result<()> {
        if name.trim().is_empty() {
            return Err(StoreError("Nome vazio".into()));
        }
        if self.products.contains_key(&id) {
            return Err(StoreError("Produto ja cadastrado".into()));
        }
        self.products.insert(
            id,
            Product {
                id,
                name: name.trim().to_owned(),
            },
        );
        self.adjacency.insert(Vertex::Product(id), Vec::new());
        Ok(())
    }
    pub fn add_customer(&mut self, id: u32) {
        self.adjacency.entry(Vertex::Customer(id)).or_default();
    }
    pub fn add_category(&mut self, id: u32) {
        self.adjacency.entry(Vertex::Category(id)).or_default();
    }
    pub fn product(&self, id: u32) -> Option<&Product> {
        self.products.get(&id)
    }
    pub fn products(&self) -> Vec<&Product> {
        let mut items: Vec<_> = self.products.values().collect();
        items.sort_by_key(|p| p.id);
        items
    }
    pub fn neighbors(&self, v: Vertex) -> Option<&[Edge]> {
        self.adjacency.get(&v).map(Vec::as_slice)
    }
    pub fn vertex_count(&self) -> usize {
        self.adjacency.len()
    }
    pub fn edge_count(&self) -> usize {
        self.adjacency.values().map(Vec::len).sum::<usize>() / 2
    }

    /// Uma aresta por par e tipo; reconectar atualiza o peso em ambos os sentidos.
    pub fn connect(&mut self, a: Vertex, b: Vertex, relation: Relation, weight: f64) -> Result<()> {
        use Relation::*;
        use Vertex::*;
        if a == b {
            return Err(StoreError("Lacos nao permitidos".into()));
        }
        if !weight.is_finite() || weight <= 0.0 || weight > 1.0 {
            return Err(StoreError("Peso deve estar em (0, 1]".into()));
        }
        let valid = match relation {
            Purchase => matches!(
                (a, b),
                (Customer(_), Product(_)) | (Product(_), Customer(_))
            ),
            Interest => matches!(
                (a, b),
                (Customer(_), Product(_))
                    | (Product(_), Customer(_))
                    | (Customer(_), Category(_))
                    | (Category(_), Customer(_))
            ),
            BelongsTo => matches!(
                (a, b),
                (Product(_), Category(_)) | (Category(_), Product(_))
            ),
            Similarity => matches!((a, b), (Product(_), Product(_))),
        };
        if !valid {
            return Err(StoreError("Relacao incompativel com os vertices".into()));
        }
        if !self.adjacency.contains_key(&a) || !self.adjacency.contains_key(&b) {
            return Err(StoreError("Vertice nao cadastrado".into()));
        }
        for (from, to) in [(a, b), (b, a)] {
            let edges = self.adjacency.get_mut(&from).unwrap();
            if let Some(e) = edges
                .iter_mut()
                .find(|e| e.to == to && e.relation == relation)
            {
                e.weight = weight;
            } else {
                edges.push(Edge {
                    to,
                    relation,
                    weight,
                });
            }
        }
        Ok(())
    }
    pub fn disconnect(&mut self, a: Vertex, b: Vertex, relation: Relation) -> Result<bool> {
        if !self.adjacency.contains_key(&a) || !self.adjacency.contains_key(&b) {
            return Err(StoreError("Vertice nao cadastrado".into()));
        }
        let mut removed = false;
        for (from, to) in [(a, b), (b, a)] {
            let edges = self.adjacency.get_mut(&from).unwrap();
            let before = edges.len();
            edges.retain(|e| !(e.to == to && e.relation == relation));
            removed |= before != edges.len();
        }
        Ok(removed)
    }
    /// BFS determina distancias; soma produtos dos pesos dos caminhos minimos.
    /// Distintos tipos de aresta sao evidencias distintas. Nao usa caminhos com ciclos.
    pub fn recommend(
        &self,
        source: Vertex,
        max_depth: usize,
        limit: usize,
    ) -> Result<Vec<Recommendation>> {
        let edges = self
            .adjacency
            .get(&source)
            .ok_or_else(|| StoreError("Origem inexistente".into()))?;
        let mut excluded = HashSet::new();
        if let Vertex::Product(id) = source {
            excluded.insert(id);
        }
        if matches!(source, Vertex::Customer(_)) {
            for e in edges {
                if e.relation == Relation::Purchase {
                    if let Vertex::Product(id) = e.to {
                        excluded.insert(id);
                    }
                }
            }
        }
        let mut distance = HashMap::from([(source, 0usize)]);
        let mut evidence = HashMap::from([(source, 1.0f64)]);
        let mut queue = VecDeque::from([source]);
        while let Some(v) = queue.pop_front() {
            let d = distance[&v];
            if d >= max_depth {
                continue;
            }
            let value = evidence[&v];
            for edge in &self.adjacency[&v] {
                let next = d + 1;
                if let std::collections::hash_map::Entry::Vacant(entry) = distance.entry(edge.to) {
                    entry.insert(next);
                    queue.push_back(edge.to);
                }
                if distance[&edge.to] == next {
                    *evidence.entry(edge.to).or_insert(0.0) += value * edge.weight;
                }
            }
        }
        let mut result: Vec<_> = distance
            .iter()
            .filter_map(|(v, d)| {
                if let Vertex::Product(id) = v {
                    if *d > 0 && !excluded.contains(id) {
                        return Some(Recommendation {
                            product_id: *id,
                            score: evidence[v] / *d as f64,
                            distance: *d,
                        });
                    }
                }
                None
            })
            .collect();
        result.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then(a.product_id.cmp(&b.product_id))
        });
        result.truncate(limit);
        Ok(result)
    }
}

pub fn demo_store() -> Store {
    use Relation::*;
    use Vertex::*;
    let mut s = Store::new();
    for (id, name) in [
        (1, "Notebook"),
        (2, "Mouse"),
        (3, "Teclado"),
        (4, "Livro de Rust"),
        (5, "Monitor"),
    ] {
        s.add_product(id, name).unwrap();
    }
    s.add_customer(1);
    s.add_customer(2);
    s.add_category(1);
    s.add_category(2);
    for id in [1, 2, 3, 5] {
        s.connect(Product(id), Category(1), BelongsTo, 1.0).unwrap();
    }
    s.connect(Product(4), Category(2), BelongsTo, 1.0).unwrap();
    for (c, p) in [(1, 1), (2, 1), (2, 2), (2, 3)] {
        s.connect(Customer(c), Product(p), Purchase, 1.0).unwrap();
    }
    s.connect(Customer(1), Category(1), Interest, 0.8).unwrap();
    s.connect(Product(1), Product(2), Similarity, 0.9).unwrap();
    s.connect(Product(1), Product(5), Similarity, 0.7).unwrap();
    s
}

/// Base sintetica deterministica esparsa, sem dependencias externas.
pub fn synthetic_store(n: u32) -> Store {
    use Relation::*;
    use Vertex::*;
    let mut s = Store::new();
    for c in 0..100 {
        s.add_category(c);
    }
    for p in 0..n {
        s.add_product(p, &format!("Produto {p}")).unwrap();
    }
    for p in 0..n {
        s.connect(Product(p), Category(p % 100), BelongsTo, 1.0)
            .unwrap();
        if p + 1 < n {
            s.connect(Product(p), Product(p + 1), Similarity, 0.7)
                .unwrap();
        }
    }
    for c in 0..(n / 5).max(1) {
        s.add_customer(c);
        for j in 0..5 {
            if n > 0 {
                s.connect(Customer(c), Product((c * 5 + j) % n), Purchase, 1.0)
                    .unwrap();
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registration() {
        let mut s = Store::new();
        assert!(s.add_product(1, "Mouse").is_ok());
        assert!(s.add_product(1, "Outro").is_err());
        assert!(s.add_product(2, " ").is_err());
        assert_eq!(s.product(1).unwrap().name, "Mouse");
        assert!(s.product(99).is_none());
    }
    #[test]
    fn edge_validation_and_update() {
        let mut s = demo_store();
        let before = s.edge_count();
        s.connect(
            Vertex::Product(1),
            Vertex::Product(2),
            Relation::Similarity,
            0.4,
        )
        .unwrap();
        assert_eq!(before, s.edge_count());
        for (a, b) in [(1, 2), (2, 1)] {
            assert_eq!(
                s.neighbors(Vertex::Product(a))
                    .unwrap()
                    .iter()
                    .find(|e| e.to == Vertex::Product(b))
                    .unwrap()
                    .weight,
                0.4
            );
        }
        for w in [0.0, -1.0, 1.1, f64::NAN, f64::INFINITY] {
            assert!(s
                .connect(
                    Vertex::Product(1),
                    Vertex::Product(2),
                    Relation::Similarity,
                    w
                )
                .is_err());
        }
        assert!(s
            .connect(
                Vertex::Customer(1),
                Vertex::Category(1),
                Relation::Purchase,
                1.0
            )
            .is_err());
        assert!(s
            .connect(
                Vertex::Product(1),
                Vertex::Product(99),
                Relation::Similarity,
                1.0
            )
            .is_err());
        assert!(s
            .connect(
                Vertex::Product(1),
                Vertex::Product(1),
                Relation::Similarity,
                1.0
            )
            .is_err());
    }
    #[test]
    fn removal_is_symmetric() {
        let mut s = demo_store();
        let before = s.edge_count();
        assert!(s
            .disconnect(Vertex::Product(1), Vertex::Product(2), Relation::Similarity)
            .unwrap());
        assert_eq!(s.edge_count(), before - 1);
        assert!(!s
            .disconnect(Vertex::Product(2), Vertex::Product(1), Relation::Similarity)
            .unwrap());
        assert!(s
            .disconnect(
                Vertex::Product(99),
                Vertex::Product(1),
                Relation::Similarity
            )
            .is_err());
    }
    #[test]
    fn shortest_paths_aggregate_and_ties_are_stable() {
        use Relation::*;
        use Vertex::*;
        let mut s = Store::new();
        s.add_customer(1);
        s.add_category(1);
        s.add_category(2);
        for p in 1..=2 {
            s.add_product(p, "P").unwrap();
        }
        for c in 1..=2 {
            s.connect(Customer(1), Category(c), Interest, 0.8).unwrap();
            for p in 1..=2 {
                s.connect(Category(c), Product(p), BelongsTo, 1.0).unwrap();
            }
        }
        let r = s.recommend(Customer(1), 3, 10).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].product_id, 1);
        assert_eq!(r[0].distance, 2);
        assert!((r[0].score - 0.8).abs() < 1e-10);
    }
    #[test]
    fn boundaries_and_disconnected() {
        let mut s = demo_store();
        s.add_customer(3);
        assert!(s.recommend(Vertex::Customer(3), 3, 10).unwrap().is_empty());
        assert!(s.recommend(Vertex::Customer(1), 0, 10).unwrap().is_empty());
        assert!(s.recommend(Vertex::Customer(1), 3, 0).unwrap().is_empty());
        assert!(s.recommend(Vertex::Customer(99), 3, 10).is_err());
    }
    #[test]
    fn purchased_products_and_duplicates_are_excluded() {
        let s = demo_store();
        let r = s.recommend(Vertex::Customer(1), 3, 20).unwrap();
        assert!(!r.iter().any(|r| r.product_id == 1));
        assert_eq!(
            r.len(),
            r.iter().map(|r| r.product_id).collect::<HashSet<_>>().len()
        );
        assert_eq!(s.recommend(Vertex::Customer(1), 3, 1).unwrap().len(), 1);
        assert!(!s
            .recommend(Vertex::Product(1), 3, 20)
            .unwrap()
            .iter()
            .any(|r| r.product_id == 1));
    }
}
