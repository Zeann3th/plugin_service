IMAGE_NAME = plugin-service
CONTAINER_NAME = plugin-service
PORT = 7554

.PHONY: build run stop restart logs clean

build:
	docker build -t $(IMAGE_NAME) .

run:
	docker run --env-file .env \
		-p $(PORT):$(PORT) \
		--name $(CONTAINER_NAME) \
		-d \
		$(IMAGE_NAME)

stop:
	docker stop $(CONTAINER_NAME)
	docker rm $(CONTAINER_NAME)

restart: stop run

logs:
	docker logs -f $(CONTAINER_NAME)

clean: stop
	docker rmi $(IMAGE_NAME)

dev: build run logs
