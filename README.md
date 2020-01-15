# About

Queries is a Gtk-based graphical SQL client supporting PostgreSQL, Sqlite3 and CSV files.

# CSV support

CSV files can be opened with Queries, but recall that CSV and SQL are representing different things: CSV is an ordered sequence of records, while the result of a SQL query is an unordered sequence of records. That said, the table that you see happens to have the same ordering of the file. Every time you open a CSV file in the application, think of the following query being invoked on your behalf, which includes an invisible column "index" populated with the order of records in the file:

```sql
select * from table order by index;
```

If your query logic requires indexing records (which is a common operation if you are used to CSV manipulation tools such as awk), you should have a similar "index" column in your table; if you logic requires indexing regions of data, you should also have a column discriminating such regions. Another difference is that although CSV is untyped, the loaded representation is typed. The application uploads the set of CSV files that you loaded into a in-memory SQlite3 database, so any of the logic described in the [SQlite3 documentation](https://www.sqlite.org/csv.html) also applies here.

# Numerical computing functionality 

Statistical modelling can be mostly done in a declarative fashion, so Queries supports some limited set of statistical functionality beyond what is offered by the basic SQL engines. If your tables live in the Local SQL engine, some functions are made available in addition to the ones offered by the standard set of Sqlite3 functions. This functionality relies on the well-establihsed GSL C implementations of linear and non-linear regression routines. R-style formulas follows standard syntax and rely on `nom` for parsing. A slight abuse is introduced so that factors can always be decomposed into B-splines, wavelets and other basis functions, the use of which can model non-linear time-series processes.

**Univariate model fitting functions**

```
mle_normal(table.*, "model1", "y ~ 1 + x")
mle_binomial(table.*, "model2", "y ~ 1 + x")
mle_poisson(table.*, "model3", "y ~ 1 + x")
```

**Multivariate model fitting functions**

Multivariate functions require a group column to disambiguate different samples.

```
mvn_fit(table.age, table.group, "mv_1", "y ~ x")
```

This differs from a matrix manipulation environments such as R and Matlab in that there is no notion of row ordering: Those functions abstract this detail away (since in statistics we are always concerned either with individually-exchangeable samples or group-exchangeable samples), and are treated in the same way as other aggregate functions such as `mean` or `variance`. Those functions return one column for each point estimate, variance and covariance factor, which can be further manipulated with other functions:

**Model description functions**

The fact that models are named, and that their components have standard names queryable by regular expressions, allow to filter specific rows from their description.

```
standard_error(*) 
predict(*)
residual(*)
parameter_correlation(*)
```

**Model comparison functions**

Going even further, you can compare different models, if you have outputs of model description functions at different rows:

```
likelihood_ratio("model1", "model2", *)
```

This compositional approach maps well to nested SQL queries, and is easly integrated into more long Shell scripts by relying on the `queries` command line tool and standard Unix pipes. Analysis code can live on SQL queries, while input/output and redirection can be expressable in Shell.

**Time series**

If there is a column to disambiguate temporal ordering, SQL can even be used to represent time series functionality such as linear filtering via convolutions.

```
select convolve(index, data, "{0.25, 0.5, 0.25}");
```


