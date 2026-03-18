create extension if not exists pgcrypto;

create table if not exists bookings (
    id uuid primary key,
    user_id text not null,
    flight_id uuid not null,
    passenger_name text not null,
    passenger_email text not null,
    seat_count integer not null check (seat_count > 0),
    total_price_minor_units bigint not null check (total_price_minor_units > 0),
    currency char(3) not null,
    status text not null check (status in ('CONFIRMED', 'CANCELLED')),
    created_at timestamptz not null,
    updated_at timestamptz not null
);

create index if not exists idx_bookings_user_id on bookings(user_id);
create index if not exists idx_bookings_flight_id on bookings(flight_id);
