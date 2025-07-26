mod test_utils;
use test_utils::setup_test_server;

#[tokio::test]
async fn test_full_staff_lifecycle() {
    let server = setup_test_server().await;
    
    // Create equipment
    let equipment_id = test_utils::create_test_equipment(&server).await;
    
    // Create staff
    let staff_id = test_utils::create_test_staff(&server, &[equipment_id]).await;
    
    // Update staff
    let update_response = server.post(&format!("/staff/{}", staff_id))
        .form(&[
            ("full_name", "Updated Operator"),
            ("assigned_equipment", &equipment_id.to_string()),
        ])
        .await;
    assert_eq!(update_response.status(), 303);
    
    // Delete staff
    let delete_response = server.post(&format!("/staff/{}/delete", staff_id))
        .await;
    assert_eq!(delete_response.status(), 303);
}

