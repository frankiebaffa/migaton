select count(*)
from Test.sqlite_master
where type = 'table'
and name = 'TForeigns'
and sql not like '%Name%';
