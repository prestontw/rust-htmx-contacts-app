
watch-server:
	cargo watch -x run

start-db:
	pg_ctl start -D data/ 

init-db:
	pg_ctl init -D data/
	just start-db
	createdb
	createuser postgres
	echo "alter user postgres createdb;" | psql
	diesel setup
	
