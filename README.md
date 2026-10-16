# ConectaStore

Sistema acadêmico de recomendação de produtos da MegaStore, implementado em Rust com biblioteca padrão, grafo heterogêneo e busca em largura (BFS).

## Objetivo e funcionamento

Relacionar clientes, produtos e categorias para encontrar produtos próximos aos interesses e às compras de um cliente. Também recomenda a partir de um produto. O sistema exclui compras anteriores do cliente, exclui o próprio produto de origem e entrega cada produto uma única vez.

É um protótipo local em memória: não usa banco, internet ou dependências de terceiros. Cada processo começa com a mesma base demonstrativa. Use o modo interativo para manter cadastros e alterações durante a sessão. Os dados da sessão são descartados ao sair.

## Compilação e execução

Instale Rust estável com Cargo pelo site oficial <https://www.rust-lang.org/tools/install>. No Windows, siga os pré-requisitos indicados pelo instalador para o compilador escolhido. O projeto foi validado com rustc 1.99.1, alvo x86_64-pc-windows-gnu.

Abra um terminal dentro de `megastore`:

```sh
cargo build --release
cargo run --release -- demo
cargo run --release -- interativo
cargo test
cargo run --release -- benchmark
```

Não é necessário instalar crates adicionais. `cargo run` sem argumentos também executa a demonstração.

## Exemplos de uso

Comandos avulsos, executados sobre a base demonstrativa:

```sh
cargo run --release -- listar
cargo run --release -- consultar 1
cargo run --release -- recomendar cliente 1 3 10
cargo run --release -- recomendar produto 1 2 10
cargo run --release -- grafo
```

Os dois últimos números de `recomendar` são profundidade máxima e limite de resultados. A profundidade conta arestas; o limite é aplicado depois do ranking e não reduz o trabalho de busca.

No modo interativo, execute a sequência abaixo, uma linha por vez:

```text
produto 6 Headset USB
consultar 6
cliente 3
categoria 3
conectar produto 6 categoria 3 categoria 1
conectar cliente 3 categoria 3 interesse 0.8
recomendar cliente 3 3 10
conectar cliente 3 produto 6 compra 1
recomendar cliente 3 3 10
desconectar cliente 3 produto 6 compra
recomendar produto 1 2 10
grafo
sair
```

A primeira recomendação ao cliente 3 mostra o Headset; depois da compra ele é excluído. Ao remover a compra, a recomendação volta a ser possível. Um comando inválido mostra erro e mantém a sessão aberta.

Sintaxe das conexões:

```text
conectar TIPO_A ID_A TIPO_B ID_B RELACAO PESO
desconectar TIPO_A ID_A TIPO_B ID_B RELACAO
```

Tipos: `cliente`, `produto`, `categoria`. Relações: `compra` (cliente/produto), `interesse` (cliente/produto ou cliente/categoria), `categoria` (produto/categoria) e `similaridade` (produto/produto). O peso deve ser finito e estar em (0, 1]. Use ponto decimal. Os vértices devem existir antes da conexão. Reconectar o mesmo par e tipo atualiza o peso; laços e combinações inválidas são rejeitados. Cadastro de cliente/categoria é idempotente; produto com ID repetido retorna erro.

Resultado da demonstração para o cliente 1:

```text
2 | Mouse | score=0.8500 | distancia=2
5 | Monitor | score=0.7500 | distancia=2
3 | Teclado | score=0.4000 | distancia=2
```

O notebook comprado não aparece. O livro está desconectado do cliente e também não aparece.

## Arquitetura e estruturas

```text
src/main.rs       CLI, demonstração e benchmark
src/lib.rs        Store, validação, grafo, BFS, ranking e dados sintéticos
tests/workflow.rs testes de integração da API e do executável
resultados/       medidas reais e resumo da validação
Cargo.toml        configuração sem dependências externas
Cargo.lock        resolução reproduzível do projeto
```

`HashMap<u32, Product>` permite consulta esperada O(1) por ID. `HashMap<Vertex, Vec<Edge>>` armazena listas de adjacência. `Vertex` distingue os três tipos de entidade, mesmo quando usam o mesmo ID numérico. Cada aresta lógica tem dois registros, um por sentido. Tipos distintos podem coexistir no mesmo par; não há duas arestas do mesmo tipo no mesmo par. Portanto, é um multigrafo tipado, ponderado, não direcionado, sem laços.

`VecDeque` é a fila BFS; mapas guardam distância e evidência; `HashSet` guarda produtos excluídos; `Vec` guarda e ordena os candidatos. Não há matriz de adjacência: a lista usa O(V+E), enquanto uma matriz requer O(V²). Na lista, verificar/atualizar/remover uma conexão custa O(grau(a)+grau(b)), pois os vetores são percorridos.

## Algoritmo e pontuação

1. Validar a origem e reunir os produtos excluídos.
2. Executar BFS até a profundidade máxima, visitando cada vértice uma vez.
3. Acumular a evidência somente por arestas que avançam para o próximo nível BFS: `e[v] += e[u] * peso(u,v)`, com `e[origem]=1`.
4. Para cada produto elegível, calcular `score=e[produto]/distância`.
5. Ordenar por score decrescente, desempatar por ID crescente e aplicar o limite.

A fila garante que todos os predecessores do nível anterior sejam processados antes de expandir um vértice. Assim, somam-se os produtos dos pesos de todos os caminhos mínimos sem enumerá-los. Ciclos e caminhos mais longos não aumentam o score. A BFS mede saltos; os pesos representam força de evidência, não custo de caminho. O score não é probabilidade e pode ser maior que 1. Relações de tipos diferentes contam como evidências distintas. A soma favorece popularidade, uma limitação conhecida. Não há garantia de igualdade bit a bit em somas de ponto flutuante após mudar a ordem de inserção.

Tempo esperado por consulta: O(grau(origem)+Vd+Ed+C log C), onde Vd são os vértices descobertos, Ed são as entradas de adjacência examinadas e C os produtos candidatos. Espaço auxiliar O(grau(origem)+Vd+C), além de O(V+E) do grafo e do texto dos nomes. No pior caso, o percurso alcança todo o grafo. Os custos dos mapas são esperados; alocações e colisões podem alterar constantes e casos extremos.

## Testes

`cargo test` executou **9 testes: 6 unitários e 3 de integração, todos aprovados**. Há validação de cadastro, consulta ausente, pesos inválidos, relação incompatível, aresta atualizada, remoção simétrica, ciclos, acúmulo de caminhos mínimos, empate, isolamento, profundidade/limite zero, origem inexistente, exclusão de comprados, ausência de duplicatas, fluxo colaborativo e CLI real com sucesso e erro. O arquivo `resultados/validacao.txt` registra o resumo da execução.

## Resultados de desempenho

Execução real em Windows x64, AMD Ryzen 7 5700G, Rust 1.99.1, compilação `--release`. Valores de uma rodada local; não são garantia de produção. As consultas não incluem impressão em terminal ou construção do grafo.

| Produtos | Vértices | Arestas lógicas | Construção ms | Mediana ms | p95 ms |
|---:|---:|---:|---:|---:|---:|
| 1.000 | 1.300 | 2.999 | 0,7102 | 0,0129 | 0,0140 |
| 10.000 | 12.100 | 29.999 | 10,1173 | 0,0910 | 0,1541 |
| 50.000 | 60.100 | 149.999 | 58,6340 | 0,4169 | 0,4228 |

Metodologia: 100 categorias, n produtos, max(1,n/5) clientes, cinco compras por cliente, uma categoria por produto e similaridade entre produtos consecutivos. Cinco consultas de aquecimento e 31 consultas medidas por tamanho, clientes 0 a 30, profundidade 3 e top 10. Mediana: amostra ordenada 16; p95: amostra 30 (nearest rank). Usa `Instant` e `black_box`; cada tamanho retornou 310 resultados no total. A topologia é determinística, mas o hash aleatório, o sistema e o hardware alteram tempos. Não foram medidos RSS, concorrência ou precisão de recomendações. Dados brutos em `resultados/benchmark.csv`.

A quantidade fixa de categorias faz o grau de cada categoria crescer com n. Por isso, limitar a profundidade não garante custo constante. O construtor verifica duplicatas por varredura dos vetores, podendo sofrer custo quadrático quando muitos produtos compartilham um hub. Para milhões de produtos: índices de arestas, construção em lote, CSR para trechos estáticos, controle de fan-out, cache com invalidação, processamento incremental e eventual particionamento. São evoluções propostas; não estão implementadas. Avaliar qualidade com Precision@k e Recall@k em divisão temporal antes de alegar ganho comercial.

## Relatório técnico

O PDF `relatorio_tecnico.pdf` é fornecido junto ao pacote do projeto. Ele fundamenta a implementação, compara representações e detalha complexidade, testes e escalabilidade.

## Publicação no GitHub

Crie um repositório **público** chamado `megastore` na sua conta. Dentro desta pasta, execute, substituindo `GeovaniGarcez`:

```sh
git init
git add .
git commit -m "Implementa ConectaStore em Rust com BFS e testes"
git branch -M main
git remote add origin https://github.com/GeovaniGarcez/megastore.git
git push -u origin main
```

Depois abra o endereço sem estar autenticado para conferir o acesso. `target` não será publicado. Inclua o relatório no repositório, se desejar, e envie seu link público e o PDF conforme solicitado pela disciplina.

## Vídeo pitch

**Link do vídeo: pendente de gravação e publicação pelo autor.** Substitua esta linha pelo endereço acessível antes da entrega. O vídeo é responsabilidade do autor e deve mostrar o projeto executando, com duração máxima de quatro minutos.

## Referências

- Rust Standard Library, Collections: <https://doc.rust-lang.org/std/collections/index.html>.
- The Rust Programming Language, Writing Automated Tests: <https://doc.rust-lang.org/book/ch11-00-testing.html>.

Consulte o relatório para a análise específica do algoritmo implementado.

