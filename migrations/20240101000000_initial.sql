-- Create tables for Construction Fleet Management System

-- Categories table
CREATE TABLE categories (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Staff table
CREATE TABLE staff (
    id SERIAL PRIMARY KEY,
    full_name VARCHAR(100) NOT NULL,
    contact_info TEXT,
    license_number VARCHAR(50),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Equipment table
CREATE TABLE equipment (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    brand VARCHAR(100) NOT NULL,
    model VARCHAR(100) NOT NULL,
    serial_number VARCHAR(100) UNIQUE NOT NULL,
    acquisition_date TIMESTAMPTZ NOT NULL,
    category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE RESTRICT,
    insurance_renewal TIMESTAMPTZ,
    next_maintenance TIMESTAMPTZ,
    fuel_capacity DOUBLE PRECISION,
    last_inspection TIMESTAMPTZ,
    current_status VARCHAR(20) NOT NULL DEFAULT 'active' 
        CHECK (current_status IN ('active', 'maintenance', 'retired')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Equipment Operator relationship (many-to-many)
CREATE TABLE equipment_operator (
    operator_id INTEGER NOT NULL REFERENCES staff(id) ON DELETE CASCADE,
    equipment_id INTEGER NOT NULL REFERENCES equipment(id) ON DELETE CASCADE,
    assigned_date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (operator_id, equipment_id)
);

-- Maintenance history table
CREATE TABLE maintenance_history (
    id SERIAL PRIMARY KEY,
    equipment_id INTEGER NOT NULL REFERENCES equipment(id) ON DELETE CASCADE,
    maintenance_date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    description TEXT NOT NULL,
    cost DOUBLE PRECISION,
    technician VARCHAR(100),
    next_maintenance_due TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for faster queries
CREATE INDEX idx_equipment_category ON equipment(category_id);
CREATE INDEX idx_equipment_status ON equipment(current_status);
CREATE INDEX idx_operator_equipment ON equipment_operator(operator_id, equipment_id);
CREATE INDEX idx_maintenance_equipment ON maintenance_history(equipment_id);

-- Add trigger function for updated_at timestamps
CREATE OR REPLACE FUNCTION update_modified_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create triggers for all tables with updated_at
CREATE TRIGGER update_categories_modtime 
BEFORE UPDATE ON categories 
FOR EACH ROW EXECUTE FUNCTION update_modified_column();

CREATE TRIGGER update_staff_modtime 
BEFORE UPDATE ON staff 
FOR EACH ROW EXECUTE FUNCTION update_modified_column();

CREATE TRIGGER update_equipment_modtime 
BEFORE UPDATE ON equipment 
FOR EACH ROW EXECUTE FUNCTION update_modified_column();

-- Insert sample categories
INSERT INTO categories (name) VALUES 
('Excavators'),                   -- Excavators
('Bulldozers'),                   -- Bulldozers
('Grue'),                         -- Cranes
('Bétonnière'),                   -- Concrete Mixers Mpanamory simenitra
('Générateurs'),                  -- Generators
('Forklifts'),                    -- Forklifts  chariots élévateurs
('Camions à benne'),              -- Dump Trucks.  Kamiao mpitondra entana . Camions à benne basculante
('Compactors');                   -- Compactors

-- Insert sample staff with Malagasy names
INSERT INTO staff (full_name, contact_info, license_number) VALUES 
('Rakoto Jean', 'rakoto@example.mg, +261 34 12 345 67', 'OP-MG-12345'),
('Rasoa Marie', 'rasoa@example.mg, +261 33 45 678 90', 'OP-MG-67890'),
('Rajao Nirina', 'rajao@example.mg, +261 32 11 222 33', 'OP-MG-54321'),
('Soa Voahangy', 'voahangy@example.mg, +261 38 99 888 77', 'OP-MG-09876');

-- Insert sample equipment with explicit category IDs
INSERT INTO equipment (
    name, brand, model, serial_number, acquisition_date, 
    category_id, insurance_renewal, next_maintenance, fuel_capacity, current_status
) VALUES 
('Excavator #1', 'Caterpillar', '320D', 'CAT-MG-001', '2022-01-15 08:00:00+00', 
  (SELECT id FROM categories WHERE name = 'Excavators'), 
 '2024-06-30 00:00:00+00', '2023-11-15 08:00:00+00', 250.0, 'active'),
 
('Crane Toamasina V', 'Liebherr', 'LTM 1050', 'LIE-MG-005', '2021-03-22 10:30:00+00', 
  (SELECT id FROM categories WHERE name = 'Grue'), 
 '2024-05-15 00:00:00+00', '2023-10-20 08:00:00+00', NULL, 'active'),
 
('Generator Tana VII', 'Cummins', 'C1000D5', 'CUM-MG-007', '2023-02-10 14:00:00+00', 
  (SELECT id FROM categories WHERE name = 'Générateurs'), 
 '2024-12-31 00:00:00+00', '2024-02-01 08:00:00+00', 150.0, 'active'),
 
('Dump Truck Antsirabe III', 'Volvo', 'A40G', 'VOL-MG-003', '2020-11-05 09:15:00+00', 
  (SELECT id FROM categories WHERE name = 'Camions à benne'), 
 '2023-09-30 00:00:00+00', '2023-08-15 08:00:00+00', 300.0, 'maintenance');

-- Assign operators to equipment
INSERT INTO equipment_operator (operator_id, equipment_id) VALUES
((SELECT id FROM staff WHERE full_name = 'Rakoto Jean'), 
 (SELECT id FROM equipment WHERE name = 'Excavator #1')),
 
((SELECT id FROM staff WHERE full_name = 'Rasoa Marie'), 
 (SELECT id FROM equipment WHERE name = 'Crane Toamasina V')),
 
((SELECT id FROM staff WHERE full_name = 'Rajao Nirina'), 
 (SELECT id FROM equipment WHERE name = 'Dump Truck Antsirabe III'));


-- Insert sample maintenance history
INSERT INTO maintenance_history (
    equipment_id, maintenance_date, description, cost, technician, next_maintenance_due
) VALUES
((SELECT id FROM equipment WHERE name = 'Dump Truck Antsirabe III'), 
 '2023-07-20 09:00:00+00', 'Engine overhaul and transmission service', 4250.75, 
 'Randria Jean', '2023-08-15 08:00:00+00'),
 
((SELECT id FROM equipment WHERE name = 'Excavator #1'), 
 '2023-09-10 10:30:00+00', 'Hydraulic system maintenance', 1820.50, 
 'Service TechPro', '2023-11-15 08:00:00+00');

