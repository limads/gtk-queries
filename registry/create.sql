drop table if exists library;

drop table if exists function;

drop table if exists arg;

drop table if exists ret;

create table library (
    id integer,
    name text,
    libpath text,
    srcpath text,
    active integer
);

create table function(
    id integer primary key,
    lib_id integer references library(id) on delete cascade,
    name text,
    doc text,
    fn_mode text,
    var_arg integer,
    var_ret integer
);

create table arg(
    fn_id integer references function(id) on delete cascade,
    pos integer,
    type text
);

create table ret(
    fn_id integer references function(id) on delete cascade,
    pos integer,
    type text
);


