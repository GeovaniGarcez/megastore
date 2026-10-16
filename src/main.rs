use megastore::{demo_store, synthetic_store, Relation, Store, Vertex};
use std::{
    error::Error,
    io::{self, Write},
    time::Instant,
};

fn vertex(kind: &str, id: &str) -> Result<Vertex, Box<dyn Error>> {
    let id = id.parse()?;
    Ok(match kind {
        "cliente" => Vertex::Customer(id),
        "produto" => Vertex::Product(id),
        "categoria" => Vertex::Category(id),
        _ => return Err("Tipo: cliente, produto ou categoria".into()),
    })
}
fn relation(s: &str) -> Result<Relation, Box<dyn Error>> {
    Ok(match s {
        "compra" => Relation::Purchase,
        "interesse" => Relation::Interest,
        "categoria" => Relation::BelongsTo,
        "similaridade" => Relation::Similarity,
        _ => return Err("Relacao desconhecida".into()),
    })
}
fn show(s: &Store, v: Vertex, depth: usize, limit: usize) -> Result<(), Box<dyn Error>> {
    let r = s.recommend(v, depth, limit)?;
    if r.is_empty() {
        println!("Nenhuma recomendacao encontrada.");
    }
    for item in r {
        println!(
            "{} | {} | score={:.4} | distancia={}",
            item.product_id,
            s.product(item.product_id).unwrap().name,
            item.score,
            item.distance
        );
    }
    Ok(())
}
fn execute(s: &mut Store, line: &str) -> Result<(), Box<dyn Error>> {
    let a: Vec<_> = line.split_whitespace().collect();
    match a.as_slice() {
        ["listar"] => {
            for p in s.products() {
                println!("{} | {}", p.id, p.name);
            }
        }
        ["consultar", id] => match s.product(id.parse()?) {
            Some(p) => println!("{} | {}", p.id, p.name),
            None => return Err("Produto nao encontrado".into()),
        },
        ["produto", id, name @ ..] if !name.is_empty() => {
            s.add_product(id.parse()?, &name.join(" "))?;
            println!("Produto cadastrado.");
        }
        ["cliente", id] => {
            s.add_customer(id.parse()?);
            println!("Cliente cadastrado.");
        }
        ["categoria", id] => {
            s.add_category(id.parse()?);
            println!("Categoria cadastrada.");
        }
        ["conectar", ka, ia, kb, ib, r, w] => {
            s.connect(vertex(ka, ia)?, vertex(kb, ib)?, relation(r)?, w.parse()?)?;
            println!("Conexao registrada.");
        }
        ["desconectar", ka, ia, kb, ib, r] => {
            println!(
                "Removida: {}",
                s.disconnect(vertex(ka, ia)?, vertex(kb, ib)?, relation(r)?)?
            );
        }
        ["recomendar", kind, id, depth, limit] => {
            show(s, vertex(kind, id)?, depth.parse()?, limit.parse()?)?
        }
        ["grafo"] => println!("{} vertices; {} arestas", s.vertex_count(), s.edge_count()),
        _ => return Err("Comando invalido. Consulte o README.".into()),
    }
    Ok(())
}
fn benchmark() {
    println!("produtos,vertices,arestas,construcao_ms,consultas,mediana_ms,p95_ms,resultados");
    for n in [1_000, 10_000, 50_000] {
        let start = Instant::now();
        let s = synthetic_store(n);
        let build = start.elapsed().as_secs_f64() * 1000.0;
        for c in 0..5 {
            std::hint::black_box(s.recommend(Vertex::Customer(c), 3, 10).unwrap());
        }
        let mut samples = Vec::new();
        let mut total = 0;
        for c in 0..31 {
            let start = Instant::now();
            let result = s.recommend(Vertex::Customer(c), 3, 10).unwrap();
            total += result.len();
            std::hint::black_box(result);
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "{n},{},{},{build:.6},31,{:.6},{:.6},{total}",
            s.vertex_count(),
            s.edge_count(),
            samples[15],
            samples[29]
        );
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut s = demo_store();
    match args.first().map(String::as_str) {
        Some("benchmark") => benchmark(),
        Some("demo") | None => {
            println!("ConectaStore - cliente 1");
            show(&s, Vertex::Customer(1), 3, 10)?;
            println!("Produtos relacionados ao notebook");
            show(&s, Vertex::Product(1), 2, 10)?;
        }
        Some("interativo") => {
            println!("ConectaStore - base demonstrativa em memoria. Digite sair para encerrar.");
            loop {
                print!("> ");
                io::stdout().flush()?;
                let mut line = String::new();
                if io::stdin().read_line(&mut line)? == 0 || line.trim() == "sair" {
                    break;
                }
                if let Err(e) = execute(&mut s, line.trim()) {
                    println!("Erro: {e}");
                }
            }
        }
        _ => execute(&mut s, &args.join(" "))?,
    }
    Ok(())
}
