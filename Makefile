.PHONY: monitor clean

monitor: ## Compila en modo de alta optimización y arranca la interfaz htop
	@cargo run --release

clean: ## Purga de forma radical toda la basura temporal generada por Cargo y Arch
	@cargo clean
	@rm -rf pkg/ src/gnome-ext-hanabi src/build target/ *.pkg.tar.zst
	@echo "🧹 Entorno local completamente higienizado."
