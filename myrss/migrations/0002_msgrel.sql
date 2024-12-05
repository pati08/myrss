CREATE TABLE IF NOT EXISTS room_messages (
  room INTEGER REFERENCES rooms(id),
  message INTEGER REFERENCES messages(id)
)
