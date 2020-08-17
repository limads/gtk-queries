drop table if exists library;

drop table if exists function;

drop table if exists arg;

drop table if exists aggregate;

create table library (
    id integer primary key,
    name text unique,
    libpath text,
    srcpath text,
    active integer
);

create table function(
    id integer primary key,
    lib_id integer references library(id) on delete cascade,
    name text,
    doc text,
    ret text,
    var_arg integer
);

create table arg(
    fn_id integer references function(id) on delete cascade,
    pos integer,
    type text
);

create table aggregate(
    id integer primary key,
    lib_id integer references library(id) on delete cascade,
    name text,
    init integer references function(id),
    state integer references function(id),
    final integer references function(id)
);



