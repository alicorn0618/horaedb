// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Partitioned locks

use std::{
    hash::{BuildHasher, Hash},
    sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

/// Simple partitioned `RwLock`
pub struct PartitionedRwLock<T, B>
where
    B: BuildHasher,
{
    partitions: Vec<RwLock<T>>,
    partition_mask: usize,
    hash_builder: B,
}

impl<T, B> PartitionedRwLock<T, B>
where
    B: BuildHasher,
{
    /// New cache with capacity set to `2^bit_len`
    pub fn try_new_with_bit_len<F, E>(
        init_fn: F,
        partition_bit_len: usize,
        hash_builder: B,
    ) -> Result<Self, E>
    where
        F: Fn(usize) -> Result<T, E>,
    {
        let partition_num = 1 << partition_bit_len;
        PartitionedRwLock::try_new(init_fn, partition_num, hash_builder)
    }

    /// New cache with capacity round to `suggest_num`'s power of 2
    pub fn try_new_with_suggest_num<F, E>(
        init_fn: F,
        suggest_num: usize,
        hash_builder: B,
    ) -> Result<Self, E>
    where
        F: Fn(usize) -> Result<T, E>,
    {
        let partition_num = suggest_num.next_power_of_two();
        PartitionedRwLock::try_new(init_fn, partition_num, hash_builder)
    }

    pub fn read<K: Eq + Hash>(&self, key: &K) -> RwLockReadGuard<'_, T> {
        let rwlock = self.get_partition(key);

        rwlock.read().unwrap()
    }

    pub fn write<K: Eq + Hash>(&self, key: &K) -> RwLockWriteGuard<'_, T> {
        let rwlock = self.get_partition(key);

        rwlock.write().unwrap()
    }

    fn get_partition<K: Eq + Hash>(&self, key: &K) -> &RwLock<T> {
        &self.partitions[(self.hash_builder.hash_one(key) as usize) & self.partition_mask]
    }

    #[inline]
    fn try_new<F, E>(init_fn: F, partition_num: usize, hash_builder: B) -> Result<Self, E>
    where
        F: Fn(usize) -> Result<T, E>,
    {
        let partitions = (0..partition_num)
            .map(|_| init_fn(partition_num).map(RwLock::new))
            .collect::<Result<Vec<RwLock<T>>, E>>()?;

        Ok(Self {
            partitions,
            partition_mask: partition_num - 1,
            hash_builder,
        })
    }

    #[cfg(test)]
    fn get_partition_by_index(&self, index: usize) -> &RwLock<T> {
        &self.partitions[index]
    }
}

/// Simple partitioned `Mutex`
#[derive(Debug)]
pub struct PartitionedMutex<T, B>
where
    B: BuildHasher,
{
    partitions: Vec<Mutex<T>>,
    partition_mask: usize,
    hash_builder: B,
}

impl<T, B> PartitionedMutex<T, B>
where
    B: BuildHasher,
{
    /// New cache with capacity set to `2^bit_len`
    pub fn try_new_with_bit_len<F, E>(
        init_fn: F,
        partition_bit_len: usize,
        hash_builder: B,
    ) -> Result<Self, E>
    where
        F: Fn(usize) -> Result<T, E>,
    {
        let partition_num = 1 << partition_bit_len;
        PartitionedMutex::try_new(init_fn, partition_num, hash_builder)
    }

    /// New cache with capacity round to `suggest_num`'s power of 2
    pub fn try_new_with_suggest_num<F, E>(
        init_fn: F,
        suggest_num: usize,
        hash_builder: B,
    ) -> Result<Self, E>
    where
        F: Fn(usize) -> Result<T, E>,
    {
        let partition_num = suggest_num.next_power_of_two();
        PartitionedMutex::try_new(init_fn, partition_num, hash_builder)
    }

    pub fn lock<K: Eq + Hash>(&self, key: &K) -> MutexGuard<'_, T> {
        let mutex = self.get_partition(key);

        mutex.lock().unwrap()
    }

    fn get_partition<K: Eq + Hash>(&self, key: &K) -> &Mutex<T> {
        &self.partitions[(self.hash_builder.hash_one(key) as usize) & self.partition_mask]
    }

    #[inline]
    fn try_new<F, E>(init_fn: F, partition_num: usize, hash_builder: B) -> Result<Self, E>
    where
        F: Fn(usize) -> Result<T, E>,
    {
        let partitions = (0..partition_num)
            .map(|_| init_fn(partition_num).map(Mutex::new))
            .collect::<Result<Vec<Mutex<T>>, E>>()?;

        Ok(Self {
            partitions,
            partition_mask: partition_num - 1,
            hash_builder,
        })
    }

    #[cfg(test)]
    fn get_partition_by_index(&self, index: usize) -> &Mutex<T> {
        &self.partitions[index]
    }

    /// This function should be marked with `#[cfg(test)]`, but there is [an issue](https://github.com/rust-lang/cargo/issues/8379) in cargo, so public this function now.
    pub fn get_all_partition(&self) -> &Vec<Mutex<T>> {
        &self.partitions
    }
}

#[derive(Debug)]
pub struct PartitionedMutexAsync<T, B>
where
    B: BuildHasher,
{
    partitions: Vec<tokio::sync::Mutex<T>>,
    partition_mask: usize,
    hash_builder: B,
}

impl<T, B> PartitionedMutexAsync<T, B>
where
    B: BuildHasher,
{
    /// New cache with capacity set to `2^bit_len`
    pub fn try_new_with_bit_len<F, E>(
        init_fn: F,
        partition_bit_len: usize,
        hash_builder: B,
    ) -> Result<Self, E>
    where
        F: Fn(usize) -> Result<T, E>,
    {
        let partition_num = 1 << partition_bit_len;
        PartitionedMutexAsync::try_new(init_fn, partition_num, hash_builder)
    }

    /// New cache with capacity round to `suggest_num`'s power of 2
    pub fn try_new_with_suggest_num<F, E>(
        init_fn: F,
        suggest_num: usize,
        hash_builder: B,
    ) -> Result<Self, E>
    where
        F: Fn(usize) -> Result<T, E>,
    {
        let partition_num = suggest_num.next_power_of_two();
        PartitionedMutexAsync::try_new(init_fn, partition_num, hash_builder)
    }

    pub async fn lock<K: Eq + Hash>(&self, key: &K) -> tokio::sync::MutexGuard<'_, T> {
        let mutex = self.get_partition(key);

        mutex.lock().await
    }

    #[inline]
    fn try_new<F, E>(init_fn: F, partition_num: usize, hash_builder: B) -> Result<Self, E>
    where
        F: Fn(usize) -> Result<T, E>,
    {
        let partitions = (0..partition_num)
            .map(|_| init_fn(partition_num).map(tokio::sync::Mutex::new))
            .collect::<Result<Vec<tokio::sync::Mutex<T>>, E>>()?;

        Ok(Self {
            partitions,
            partition_mask: partition_num - 1,
            hash_builder,
        })
    }

    fn get_partition<K: Eq + Hash>(&self, key: &K) -> &tokio::sync::Mutex<T> {
        &self.partitions[(self.hash_builder.hash_one(key) as usize) & self.partition_mask]
    }

    #[cfg(test)]
    async fn get_partition_by_index(&self, index: usize) -> &tokio::sync::Mutex<T> {
        &self.partitions[index]
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    // TODO: remove this importing.
    use hash_ext::{build_fixed_seed_ahasher_builder, SeaHasherBuilder};

    use super::*;

    #[test]
    fn test_new_equivalence() {
        let init_42 = |_: usize| Ok::<_, ()>(42);

        let test_rwlock_42_bit_len =
            PartitionedRwLock::try_new_with_bit_len(init_42, 4, build_fixed_seed_ahasher_builder())
                .unwrap();
        let test_rwlock_42_suggest_num = PartitionedRwLock::try_new_with_suggest_num(
            init_42,
            13,
            build_fixed_seed_ahasher_builder(),
        )
        .unwrap();

        let test_mutex_42_bit_len =
            PartitionedMutex::try_new_with_bit_len(init_42, 4, build_fixed_seed_ahasher_builder())
                .unwrap();
        let test_mutex_42_suggest_num = PartitionedMutex::try_new_with_suggest_num(
            init_42,
            16,
            build_fixed_seed_ahasher_builder(),
        )
        .unwrap();

        let test_mutex_async_42_bit_len = PartitionedMutexAsync::try_new_with_bit_len(
            init_42,
            4,
            build_fixed_seed_ahasher_builder(),
        )
        .unwrap();
        let test_mutex_async_42_suggest_num = PartitionedMutexAsync::try_new_with_suggest_num(
            init_42,
            13,
            build_fixed_seed_ahasher_builder(),
        )
        .unwrap();

        assert_eq!(
            test_rwlock_42_bit_len.partition_mask,
            test_rwlock_42_suggest_num.partition_mask
        );
        assert_eq!(
            test_mutex_42_bit_len.partition_mask,
            test_mutex_42_suggest_num.partition_mask
        );
        assert_eq!(
            test_mutex_async_42_bit_len.partition_mask,
            test_mutex_async_42_suggest_num.partition_mask
        );
    }

    #[test]
    fn test_partitioned_rwlock() {
        let init_hmap = |_: usize| Ok::<_, ()>(HashMap::new());
        let test_locked_map = PartitionedRwLock::try_new_with_bit_len(
            init_hmap,
            4,
            build_fixed_seed_ahasher_builder(),
        )
        .unwrap();
        let test_key = "test_key".to_string();
        let test_value = "test_value".to_string();

        {
            let mut map = test_locked_map.write(&test_key);
            map.insert(test_key.clone(), test_value.clone());
        }

        {
            let map = test_locked_map.read(&test_key);
            assert_eq!(map.get(&test_key).unwrap(), &test_value);
        }
    }

    #[test]
    fn test_partitioned_mutex() {
        let init_hmap = |_: usize| Ok::<_, ()>(HashMap::new());
        let test_locked_map = PartitionedMutex::try_new_with_bit_len(
            init_hmap,
            4,
            build_fixed_seed_ahasher_builder(),
        )
        .unwrap();
        let test_key = "test_key".to_string();
        let test_value = "test_value".to_string();

        {
            let mut map = test_locked_map.lock(&test_key);
            map.insert(test_key.clone(), test_value.clone());
        }

        {
            let map = test_locked_map.lock(&test_key);
            assert_eq!(map.get(&test_key).unwrap(), &test_value);
        }
    }

    #[tokio::test]
    async fn test_partitioned_mutex_async() {
        let init_hmap = |_: usize| Ok::<_, ()>(HashMap::new());
        let test_locked_map =
            PartitionedMutexAsync::try_new_with_bit_len(init_hmap, 4, SeaHasherBuilder).unwrap();
        let test_key = "test_key".to_string();
        let test_value = "test_value".to_string();

        {
            let mut map = test_locked_map.lock(&test_key).await;
            map.insert(test_key.clone(), test_value.clone());
        }

        {
            let map = test_locked_map.lock(&test_key).await;
            assert_eq!(map.get(&test_key).unwrap(), &test_value);
        }
    }

    #[test]
    fn test_partitioned_mutex_vis_different_partition() {
        let init_vec = |_: usize| Ok::<_, ()>(Vec::<i32>::new());
        let test_locked_map =
            PartitionedMutex::try_new_with_bit_len(init_vec, 4, build_fixed_seed_ahasher_builder())
                .unwrap();
        let mutex_first = test_locked_map.get_partition_by_index(0);

        let mut _tmp_data = mutex_first.lock().unwrap();
        assert!(mutex_first.try_lock().is_err());

        let mutex_second = test_locked_map.get_partition_by_index(1);
        assert!(mutex_second.try_lock().is_ok());
        assert!(mutex_first.try_lock().is_err());
    }

    #[test]
    fn test_partitioned_rwmutex_vis_different_partition() {
        let init_vec = |_: usize| Ok::<_, ()>(Vec::<i32>::new());
        let test_locked_map = PartitionedRwLock::try_new_with_bit_len(
            init_vec,
            4,
            build_fixed_seed_ahasher_builder(),
        )
        .unwrap();
        let mutex_first = test_locked_map.get_partition_by_index(0);
        let mut _tmp = mutex_first.write().unwrap();
        assert!(mutex_first.try_write().is_err());

        let mutex_second_try_lock = test_locked_map.get_partition_by_index(1);
        assert!(mutex_second_try_lock.try_write().is_ok());
        assert!(mutex_first.try_write().is_err());
    }

    #[tokio::test]
    async fn test_partitioned_mutex_async_vis_different_partition() {
        let init_vec = |_: usize| Ok::<_, ()>(Vec::<i32>::new());
        let test_locked_map =
            PartitionedMutexAsync::try_new_with_bit_len(init_vec, 4, SeaHasherBuilder).unwrap();
        let mutex_first = test_locked_map.get_partition_by_index(0).await;

        let mut _tmp_data = mutex_first.lock().await;
        assert!(mutex_first.try_lock().is_err());

        let mutex_second = test_locked_map.get_partition_by_index(1).await;
        assert!(mutex_second.try_lock().is_ok());
        assert!(mutex_first.try_lock().is_err());
    }
}
