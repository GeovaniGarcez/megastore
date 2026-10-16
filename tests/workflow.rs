use megastore::{Relation::*, Store, Vertex::*};
use std::process::Command;

#[test]
fn complete_customer_workflow() {
    let mut s = Store::new();
    s.add_customer(7);
    s.add_customer(8);
    s.add_category(1);
    for (id, name) in [(10, "Notebook"), (20, "Mouse"), (30, "Teclado")] {
        s.add_product(id, name).unwrap();
    }
    s.connect(Customer(7), Product(10), Purchase, 1.0).unwrap();
    s.connect(Customer(8), Product(10), Purchase, 1.0).unwrap();
    s.connect(Customer(8), Product(20), Purchase, 1.0).unwrap();
    // Filtragem colaborativa: cliente -> compra -> outro cliente -> outra compra.
    let r = s.recommend(Customer(7), 3, 10).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].product_id, 20);
    assert_eq!(r[0].distance, 3);
    assert!((r[0].score - 1.0 / 3.0).abs() < 1e-10);
    s.connect(Product(10), Product(30), Similarity, 0.9)
        .unwrap();
    assert_eq!(s.recommend(Customer(7), 3, 1).unwrap()[0].product_id, 30);
    s.disconnect(Product(10), Product(30), Similarity).unwrap();
    assert_eq!(s.recommend(Customer(7), 3, 1).unwrap()[0].product_id, 20);
}

#[test]
fn real_cli_demo() {
    let output = Command::new(env!("CARGO_BIN_EXE_megastore"))
        .arg("demo")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Mouse"));
    assert!(text.contains("score="));
}

#[test]
fn real_cli_rejects_unknown_customer() {
    let output = Command::new(env!("CARGO_BIN_EXE_megastore"))
        .args(["recomendar", "cliente", "999", "3", "10"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}
