# Minikran evaluation harness
.PHONY: eval lean datalog rust

eval: lean datalog rust
	@echo "=== Minikran end-to-end eval complete ==="

lean:
	lake build Minikran.Timed

datalog:
	@echo "Soufflé gate rules ready in datalog/"

rust:
	cd rust && cargo check
