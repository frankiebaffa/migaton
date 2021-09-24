create temporary table TForeigns_bkup
	(
		Id integer
	,	Test_Id
	);
insert into TForeigns_bkup
select Id
,	Test_Id
from Test.TForeigns
order by Id asc;
drop table Test.TForeigns;
create table Test.TForeigns
	(
		Id integer primary key autoincrement
	,	Test_Id integer not null
	,	foreign key (Test_Id) references Tests (Id)
	);
insert into Test.TForeigns
select Test_Id
from TForeigns_bkup
order by Id asc;
drop table TForeigns_bkup;
