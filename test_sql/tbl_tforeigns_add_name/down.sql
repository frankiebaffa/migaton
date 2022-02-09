alter table Test.TForeigns rename to TempOldTForeigns;
create table Test.TForeigns
	(
		Id integer
	,	Test_Id
	,	foreign key (Test_Id) references Tests (Id)
	);
insert into Test.TForeigns
	(
		Id
	,	Test_Id
	)
select
	Id
,	Test_Id
from Test.TempOldTForeigns;
drop table Test.TempOldTForeigns;
