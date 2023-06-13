set positional-arguments

test:
	@cargo nextest run --all-features

changelog:
	@git cliff -o CHANGELOG.md --tag $NEW_VERSION
	@git commit -a -m "chore(release): $NEW_VERSION" || true

release version:
	@cargo release {{version}} -p turntable --execute

patch:
	@cargo release patch -p turntable --execute

echo version:
	@echo {{version}}
