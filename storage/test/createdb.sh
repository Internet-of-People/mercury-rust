dropdb testdb
createdb testdb
psql testdb -c "DROP USER IF EXISTS testuser"
psql testdb -c "CREATE USER testuser PASSWORD 'testpass'"
# su -u postgres createuser testuser
psql testdb -c "GRANT ALL PRIVILEGES ON DATABASE testdb TO testuser"

psql testdb -f test/testdb.sql

psql testdb -c "GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO testuser"
psql testdb -c "GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO testuser"
