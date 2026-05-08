CREATE KEYSPACE IF NOT EXISTS warehouse
  WITH replication = {'class': 'NetworkTopologyStrategy', 'datacenter1': 3};

CREATE TABLE IF NOT EXISTS warehouse.inventory_by_product_zone (
  product_id text,
  zone_id text,
  available_quantity int,
  reserved_quantity int,
  supplier_id text,
  last_updated timestamp,
  PRIMARY KEY (product_id, zone_id)
);

CREATE TABLE IF NOT EXISTS warehouse.inventory_by_product (
  product_id text PRIMARY KEY,
  total_available int,
  total_reserved int,
  last_updated timestamp
);

CREATE TABLE IF NOT EXISTS warehouse.inventory_by_zone (
  zone_id text,
  product_id text,
  available_quantity int,
  reserved_quantity int,
  last_updated timestamp,
  PRIMARY KEY (zone_id, product_id)
);

CREATE TABLE IF NOT EXISTS warehouse.processed_events (
  event_id text PRIMARY KEY,
  event_type text,
  processed_at timestamp
);

CREATE TABLE IF NOT EXISTS warehouse.orders (
  order_id text PRIMARY KEY,
  status text,
  created_at timestamp,
  completed_at timestamp
);

CREATE TABLE IF NOT EXISTS warehouse.order_items (
  order_id text,
  product_id text,
  zone_id text,
  quantity int,
  PRIMARY KEY (order_id, product_id, zone_id)
);
