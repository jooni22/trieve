# Makefile

default:
	@if [ ! -f .env ]; then \
		cp .env.example .env; \
		echo "File .env created based on .env.example"; \
		docker compose up -d; \
	else \
		docker compose up -d; \
	fi

down:
	docker compose down

vol_rm: down
	@docker volume ls | grep trieve | awk '{print $$2}' | while read volume; do docker volume rm $$volume; done

# Remove volumes
clean_all: down
	@docker volume ls | grep trieve | awk '{print $$2}' | while read volume; do docker volume rm $$volume; done
	@docker images | grep trieve | awk '{print $$1":"$$2}' | while read images; do docker rmi $$images; done

rebuild: down
	@docker compose build
	@docker compose up -d

# Show logs
logs:
	docker compose logs -f --tail=100

