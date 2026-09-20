all: trask lsp

trask: 
	cargo install --path .

lsp: 
	cargo install --path . --bin trask-lsp
