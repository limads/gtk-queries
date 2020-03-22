create table library (
    id integer,
    name text,
    libpath text
);

create table function(
    id integer primary key,
    lib_id integer references library(id),
    name text
    doc text
);

create table argument(
    fn_id integer references function(id),
    pos integer,
    name text
);

insert into library values (1, 'mvlearn', '/home/diego/Software/mvlearn-numeric/target/debug/libmvlearn_numeric.so');

insert into function values (1, 1, 'summary', 'Complete statistical summary of query results');

insert into argument values (1, 1, 'columns');

