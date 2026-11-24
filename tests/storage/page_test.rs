use theresav_common::{PageId, SlotId, PAGE_HEADER_SIZE, PAGE_SIZE};
use theresav_error::PageError;
use theresav_page::{Page, PageType, Tuple};

#[test]
fn test_integration_page_storage_and_recovery() {
    let mut page = Page::new(PageId::new(100), PageType::SlottedData);

    // 1. Verify fresh page parameters
    assert_eq!(page.header().page_id, PageId::new(100));
    assert_eq!(page.header().slot_count, 0);
    assert_eq!(page.contiguous_free_space(), PAGE_SIZE - PAGE_HEADER_SIZE);

    // 2. Insert multiple tuples with MVCC metadata
    let mut tids = Vec::new();
    for i in 0..10 {
        let tuple = Tuple::new(
            theresav_common::TxId::new(1),
            theresav_common::TxId::INVALID,
            0,
            1,
            vec![],
            format!("Relational row record payload #{}", i).into_bytes(),
        );
        let serialized = tuple.serialize();
        let slot = page.insert_tuple(&serialized).expect("Insert should succeed");
        tids.push((slot, tuple));
    }

    assert_eq!(page.header().slot_count, 10);

    // 3. Verify all tuples are readable and intact
    for (slot, expected_tuple) in &tids {
        let raw = page.get_tuple(*slot).expect("Tuple should be readable");
        let decoded = Tuple::deserialize(raw).expect("Tuple should decode");
        assert_eq!(&decoded, expected_tuple);
    }

    // 4. Delete odd-indexed tuples
    for i in (1..10).step_by(2) {
        let (slot, _) = tids[i];
        page.delete_tuple(slot).expect("Delete should succeed");
        assert!(matches!(page.get_tuple(slot), Err(PageError::InvalidSlot { .. })));
    }

    // 5. Compact the page and verify even-indexed tuples are intact
    let free_before_compact = page.contiguous_free_space();
    page.compact().expect("Compaction should succeed");
    let free_after_compact = page.contiguous_free_space();
    assert!(free_after_compact > free_before_compact);

    for i in (0..10).step_by(2) {
        let (slot, expected_tuple) = &tids[i];
        let raw = page.get_tuple(*slot).expect("Tuple should remain intact after compaction");
        let decoded = Tuple::deserialize(raw).expect("Tuple should decode");
        assert_eq!(&decoded, expected_tuple);
    }

    // 6. Test binary persistence and reload with checksum verification
    let serialized_page = *page.as_bytes();
    let reloaded_page = Page::from_bytes(&serialized_page).expect("Reload from disk bytes must pass");
    assert_eq!(reloaded_page.header().page_id, PageId::new(100));
}
