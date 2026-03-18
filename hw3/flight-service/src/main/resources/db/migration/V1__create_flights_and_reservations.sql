create extension if not exists pgcrypto;

create table if not exists flights (
    id uuid primary key,
    airline_code varchar(8) not null,
    flight_number varchar(16) not null,
    origin_iata char(3) not null,
    destination_iata char(3) not null,
    departure_time timestamptz not null,
    arrival_time timestamptz not null,
    total_seats integer not null check (total_seats > 0),
    available_seats integer not null check (available_seats >= 0 and available_seats <= total_seats),
    price_minor_units bigint not null check (price_minor_units > 0),
    currency char(3) not null,
    status text not null check (status in ('SCHEDULED', 'DEPARTED', 'CANCELLED', 'COMPLETED')),
    created_at timestamptz not null,
    updated_at timestamptz not null,
    unique (flight_number, departure_time)
);

create index if not exists idx_flights_route_search on flights(origin_iata, destination_iata, departure_time);

create table if not exists seat_reservations (
    id uuid primary key,
    booking_id uuid not null unique,
    flight_id uuid not null references flights(id) on delete cascade,
    seat_count integer not null check (seat_count > 0),
    status text not null check (status in ('ACTIVE', 'RELEASED', 'EXPIRED')),
    created_at timestamptz not null,
    updated_at timestamptz not null
);

create index if not exists idx_seat_reservations_flight_id on seat_reservations(flight_id);

insert into flights (
    id,
    airline_code,
    flight_number,
    origin_iata,
    destination_iata,
    departure_time,
    arrival_time,
    total_seats,
    available_seats,
    price_minor_units,
    currency,
    status,
    created_at,
    updated_at
) values
    (
        '11111111-1111-1111-1111-111111111111',
        'SU',
        'SU1001',
        'SVO',
        'LED',
        '2026-04-01T09:00:00Z',
        '2026-04-01T10:30:00Z',
        180,
        180,
        650000,
        'RUB',
        'SCHEDULED',
        now(),
        now()
    ),
    (
        '22222222-2222-2222-2222-222222222222',
        'SU',
        'SU1002',
        'SVO',
        'LED',
        '2026-04-01T18:00:00Z',
        '2026-04-01T19:30:00Z',
        180,
        180,
        690000,
        'RUB',
        'SCHEDULED',
        now(),
        now()
    ),
    (
        '33333333-3333-3333-3333-333333333333',
        'DP',
        'DP2001',
        'LED',
        'SVO',
        '2026-04-02T08:30:00Z',
        '2026-04-02T10:00:00Z',
        189,
        189,
        620000,
        'RUB',
        'SCHEDULED',
        now(),
        now()
    ),
    (
        '44444444-4444-4444-4444-444444444444',
        'FV',
        'FV3001',
        'VKO',
        'LED',
        '2026-04-01T12:15:00Z',
        '2026-04-01T13:45:00Z',
        150,
        150,
        580000,
        'RUB',
        'SCHEDULED',
        now(),
        now()
    )
on conflict (id) do nothing;
