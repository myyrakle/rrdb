dev:
	@docker-compose -f docker-compose.dev.yml up --build

buildx-deploy:
	@docker buildx build --platform linux/amd64,linux/arm64 -t myyrakle/rrdb:$(VERSION) --push  .

