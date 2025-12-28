
watch-server:
	cargo watch -x run

db-start:
	docker-compose up -d

db-stop:
	docker-compose down

db-init:
	just start-db
	# sometimes, this doesn't work because the db is still starting up.
	# Can add an `@until` script to make this work in the future.
	diesel setup

psql:
	docker-compose exec -it postgres psql -U postgres
	
db-logs:
	docker-compose logs -f postgres
