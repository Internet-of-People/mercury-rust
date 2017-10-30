sudo -u postgres dropdb testdb
sudo -u postgres createdb testdb
sudo -u postgres psql testdb -c "DROP USER IF EXISTS testuser"
sudo -u postgres psql testdb -c "CREATE USER testuser PASSWORD 'testpass'"
# sudo -u postgres createuser testuser
sudo -u postgres psql testdb -c "GRANT ALL PRIVILEGES ON DATABASE testdb TO testuser"

sudo -u postgres psql testdb -f test/testdb.sql

sudo -u postgres psql testdb -c "GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO testuser"
sudo -u postgres psql testdb -c "GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO testuser"
