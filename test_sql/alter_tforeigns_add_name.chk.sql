select count(name)
from Test.pragma_table_info('TForeigns')
where name = 'Name'
limit 1;
