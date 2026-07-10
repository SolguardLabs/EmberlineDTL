# Seguridad

## Modelo

EmberlineDTL separa la admision de rutas, la reserva de presupuesto, la ejecucion
del settlement y la conciliacion del pool. Cada ruta queda asociada a un
operador registrado, un activo primario, una reserva y un recibo final de
ejecucion.

El motor local asume que:

- los operadores ya estan registrados en el libro de operadores;
- el pool mantiene liquidez suficiente antes de admitir rutas;
- las rutas se ejecutan una sola vez;
- los reportes JSON son la superficie publica para integraciones y pruebas;
- los escenarios incluidos son deterministas.

## Invariantes Esperadas

- Las reservas cerradas conservan `paid + penalty + released == reserved`.
- El pool no mantiene saldos negativos.
- Las rutas ejecutadas enlazan recibo, reserva y operador esperados.
- Las penalizaciones quedan contabilizadas de forma separada al pago de rebates.
- Los balances de operadores reflejan los rebates pagados.

## Validaciones

La suite local ejecuta:

```bash
cargo fmt --all -- --check
cargo build --all-targets --locked
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
node scripts/check-loc.mjs
node --test "tests/node/*.test.js"
```

## Dependencias

El proyecto usa un conjunto pequeno de dependencias Rust para serializacion,
errores y hashing determinista. Dependabot esta configurado para Cargo, npm y
GitHub Actions.

## Alcance De Revision

Revisar especialmente:

- admision de rutas y limites por operador;
- calculo de reservas del pool;
- scoring de coste y latencia;
- cierre de reservas;
- consistencia entre reporte JSON y estado interno.

Los reportes de seguridad deben incluir escenario, comando ejecutado, salida JSON
relevante y una descripcion reproducible del impacto observado.
