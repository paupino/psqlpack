create table expense_status (
  id smallint not null,
  name varchar(30) not null,

  constraint pk_expense_status primary key (id) with(fillfactor = 100)
);