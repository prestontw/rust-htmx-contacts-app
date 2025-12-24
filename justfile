
watch-server:
	cargo watch -x run

start-db:
	docker-compose up -d

stop-db:
	docker-compose down

init-db:
	just start-db
	# sometimes, this doesn't work because the db is still starting up.
	# Can add an `@until` script to make this work in the future.
	diesel setup

psql:
	docker-compose exec -it postgres psql -U postgres
	
