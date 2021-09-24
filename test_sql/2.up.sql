create table Test.TForeigns
	(
		Id integer primary key autoincrement
	,	Test_Id integer not null
	,	foreign key (Test_Id) references Tests (Id)
	);
