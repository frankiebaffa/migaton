select count(name)
from Test.sqlite_master
where type = 'table'
and name = 'Tests'
limit 1;
